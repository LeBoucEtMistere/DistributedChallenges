//! This library contains all the utils you will need to communicate with the Maelstrom
//! test bench and the other nodes. This defines utils like a Message structure, and handles
//! the initialization of your nodes and the creation of interfaces to send and receive messages,
//! abstracting away the usage of the stdin and stdout and the json conversions.
//!

use std::io::{BufRead, Read, StdinLock, StdoutLock, Write};

use anyhow::Context;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

/// A message that you can send within the Maelstrom network.
///
/// This struct defines a Maelstrom message according to the [maelstrom protocol](https://github.com/jepsen-io/maelstrom/blob/main/doc/protocol.md)
/// It is generic over the type P which represents a Payload inserted in the body. For more details, see [`Body`]
///
/// ```
/// use serde::{Serialize, Deserialize};
/// use node_driver::{Message, Body};
///
/// // define a custom enum for our payload
/// #[derive(Debug, Clone, Serialize, Deserialize)]
/// #[serde(tag = "type")]
/// #[serde(rename_all = "snake_case")]
/// enum MyPayload {
///     /// one type of payload
///     V1,
/// }
/// // create a message with a payload `MyPayload::V1`
/// let message = Message {
///     src: String::from("src"),
///     dst: String::from("dst"),
///     body: Body {
///         msg_id: Some(1234),
///         in_reply_to: None,
///         payload: MyPayload::V1,
///     }
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message<P> {
    /// The identifier of the Maelstrom node that send the message
    pub src: String,
    /// The identifier of the recipient Maelstrom node
    #[serde(rename = "dest")]
    pub dst: String,
    /// The body of the message, generic over the type of payload
    pub body: Body<P>,
}

impl<P> Message<P> {
    /// Helper to build a response message from an incoming one.
    ///
    /// This will swap the original `src` and `dst` fields, and set the `in_reply_to` field to the
    /// content of the `msg_id` field in the original message.
    pub fn to_response(self, msg_id: Option<usize>, payload: P) -> Self {
        Message {
            src: self.dst,
            dst: self.src,
            body: Body {
                msg_id,
                in_reply_to: self.body.msg_id,
                payload,
            },
        }
    }
}

/// A container for the body of a [`Message`].
///
/// This defines the optional fields specified in [the protocol](https://github.com/jepsen-io/maelstrom/blob/main/doc/protocol.md) but the `type` field
/// is expected to be provided by the Payload type, which is flattened into the message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Body<P> {
    /// An optional id for the message
    pub msg_id: Option<usize>,
    /// An optional id identifying the message this one is replying to
    pub in_reply_to: Option<usize>,
    /// a payload of type P, flattened into the body. The payload is expected to provide a `type` field
    /// according to [the protocol](https://github.com/jepsen-io/maelstrom/blob/main/doc/protocol.md)
    #[serde(flatten)]
    pub payload: P,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
enum InitPayload {
    Init {
        node_id: String,
        node_ids: Vec<String>,
    },
    InitOk,
}

fn parse_msg<P>(msg: &str) -> anyhow::Result<Message<P>>
where
    P: DeserializeOwned,
{
    serde_json::from_str(msg).context("Message cannot be deserialized.")
}

/// An interface to handle receiving [`Message`] from the Maelstrom network
///
/// This handles transparently the json deserialization and the reading from stdin
pub struct InputInterface {
    stdin: StdinLock<'static>,
}

impl InputInterface {
    /// Obtain an interator over messages of type [`Message<P>`].
    ///
    /// The iterator items are [`anyhow::Result`] containing [`Message<P>`] since reading from stdin and parsing messages is a failible operation.
    pub fn iter<P>(&mut self) -> impl Iterator<Item = anyhow::Result<Message<P>>> + '_
    where
        P: DeserializeOwned,
    {
        self.stdin
            .by_ref()
            .lines()
            .map(|line_result| parse_msg(&line_result.context("Reading from stdin")?))
    }
}

impl Default for InputInterface {
    fn default() -> Self {
        Self {
            stdin: std::io::stdin().lock(),
        }
    }
}

/// An interface to handle sending `Message` to the Maelstrom network
///
/// This handles transparently the json serialization and the writing to stdout
pub struct OutputInterface {
    stdout: StdoutLock<'static>,
}

impl OutputInterface {
    /// Send a [`Message<P>`] to the malestrom Network
    ///
    /// This returns a [`anyhow::Result`] since writing to stdout if a failible operation.
    pub fn send_msg<P>(&mut self, msg: Message<P>) -> anyhow::Result<()>
    where
        P: Serialize,
    {
        serde_json::to_writer(&mut self.stdout, &msg).context("Serializing message")?;
        self.stdout
            .write_all(b"\n")
            .context("Writing trailing newline")?;
        Ok(())
    }
}

impl Default for OutputInterface {
    fn default() -> Self {
        Self {
            stdout: std::io::stdout().lock(),
        }
    }
}

/// Helper type to initialize a Maelstrom node
pub struct Maelstrom {}

impl Maelstrom {
    /// Initialize a Maelstrom node and returns useful structures to communicate with Maelstrom
    ///
    /// This handles receiving the `Init` message and responding to it, and returns a [`NodeMetadata`] instance holding informations about the Maelstrom node,
    /// as well as an [`InputInterface`] and an [`OutputInterface`] to communicate with Maelstrom.
    /// This is a failible operation since it communicates with the Maelstrom clients.
    pub fn init() -> anyhow::Result<(NodeMetadata, InputInterface, OutputInterface)> {
        let mut input = InputInterface::default();
        let init_msg: Message<InitPayload> = input
            .iter()
            .next()
            .expect("Nothing to read from stdin")
            .context("While getting init message")?;

        let mut output = OutputInterface::default();
        output
            .send_msg(Message {
                src: init_msg.dst,
                dst: init_msg.src,
                body: Body {
                    payload: InitPayload::InitOk,
                    msg_id: Some(0),
                    in_reply_to: init_msg.body.msg_id,
                },
            })
            .context("While repsonding to init message")?;

        match init_msg.body.payload {
            InitPayload::Init { node_id, node_ids } => Ok((
                NodeMetadata::new(
                    node_id.clone(),
                    node_ids
                        .iter()
                        .filter(|&nid| *nid != node_id)
                        .cloned()
                        .collect::<Vec<String>>(),
                    1,
                ),
                input,
                output,
            )),
            InitPayload::InitOk => panic!("Node should never receive an InitOk message"),
        }
    }
}

/// Holds metadata about the Maelstrom node
pub struct NodeMetadata {
    /// Id of the current Maelstrom node
    pub node_id: String,
    /// Ids of all the other nodes in the network
    pub other_nodes_ids: Vec<String>,
    next_message_id: usize,
}

impl NodeMetadata {
    /// Instantiate a new NodeMetadata object
    pub fn new(node_id: String, other_nodes_ids: Vec<String>, next_message_id: usize) -> Self {
        Self {
            node_id,
            other_nodes_ids,
            next_message_id,
        }
    }
    /// Obtain the next message id to use
    pub fn get_next_msg_id(&mut self) -> usize {
        let next_msg_id = self.next_message_id;
        self.next_message_id += 1;
        next_msg_id
    }
}
