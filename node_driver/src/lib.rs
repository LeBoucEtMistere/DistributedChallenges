use std::io::{BufRead, Read, StdinLock, StdoutLock, Write};

use anyhow::Context;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message<P> {
    pub src: String,
    #[serde(rename = "dest")]
    pub dst: String,
    pub body: Body<P>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Body<P> {
    pub msg_id: Option<usize>,
    pub in_reply_to: Option<usize>,
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

pub struct InputInterface {
    stdin: StdinLock<'static>,
}

impl InputInterface {
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

pub struct OutputInterface {
    stdout: StdoutLock<'static>,
}

impl OutputInterface {
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

pub struct Maelstrom {}

impl Maelstrom {
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

pub struct NodeMetadata {
    pub node_id: String,
    pub other_nodes_ids: Vec<String>,
    next_message_id: usize,
}

impl NodeMetadata {
    pub fn new(node_id: String, other_nodes_ids: Vec<String>, next_message_id: usize) -> Self {
        Self {
            node_id,
            other_nodes_ids,
            next_message_id,
        }
    }
    pub fn get_next_msg_id(&mut self) -> usize {
        let next_msg_id = self.next_message_id;
        self.next_message_id += 1;
        next_msg_id
    }
}
