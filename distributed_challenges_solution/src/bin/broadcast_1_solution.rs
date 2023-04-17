use std::collections::{HashMap, HashSet};

use node_driver::{Body, Maelstrom, Message, NodeMetadata};
use serde::{Deserialize, Serialize};

/// Defines the payload we want to send to clients in the broadcast challenge
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
enum BroadcastPayload {
    Topology {
        topology: HashMap<String, Vec<String>>,
    },
    TopologyOk,
    Broadcast {
        message: usize,
    },
    BroadcastOk,
    Read,
    ReadOk {
        messages: HashSet<usize>,
    },
}

/// This struct holds the internal state of our node
struct State {
    pub node_metadata: NodeMetadata,
    pub messages: HashSet<usize>,
    /// topology is optional since we don't have it when we construct State in the first place
    pub topology: Option<HashMap<String, Vec<String>>>,
}

fn main() -> anyhow::Result<()> {
    // init our node by getting its metadata and an output and input interface to communicate
    let (node_metadata, mut input, mut output) = Maelstrom::init()?;

    // build the application state
    let mut state = State {
        messages: HashSet::new(),
        node_metadata,
        topology: None, // we don't know the topology yet
    };

    // main loop: for each message we receive through the input interface (with a payload of type BroadcastPayload)
    for msg in input.iter::<BroadcastPayload>() {
        // if there was an error getting this message, propagate it (with the ? sigil)
        let msg = msg?;
        // match on the type of payload within the message, these are variants of the BroadcastPayload enum
        match msg.body.payload {
            BroadcastPayload::Topology { topology } => {
                // set the topology within our state with this data and ACK the message
                state.topology = Some(topology);
                output.send_msg(Message {
                    src: state.node_metadata.node_id.clone(),
                    dst: msg.src,
                    body: Body {
                        msg_id: Some(state.node_metadata.get_next_msg_id()),
                        in_reply_to: msg.body.msg_id,
                        payload: BroadcastPayload::TopologyOk,
                    },
                })?
            }
            BroadcastPayload::TopologyOk => {
                panic!("TopologyOk message shouldn't be received by a node")
            }
            BroadcastPayload::Broadcast { message } => {
                // add the message to our state and ACK
                state.messages.insert(message);
                output.send_msg(Message {
                    src: state.node_metadata.node_id.clone(),
                    dst: msg.src,
                    body: Body {
                        msg_id: Some(state.node_metadata.get_next_msg_id()),
                        in_reply_to: msg.body.msg_id,
                        payload: BroadcastPayload::BroadcastOk,
                    },
                })?
            }
            // we are not supposed to receive a GenerateOk message, let's panic when it happens
            BroadcastPayload::BroadcastOk { .. } => {
                panic!("BroadcastOk message shouldn't be received by a node")
            }
            BroadcastPayload::Read => output.send_msg(Message {
                // response with all our known messages
                src: state.node_metadata.node_id.clone(),
                dst: msg.src,
                body: Body {
                    msg_id: Some(state.node_metadata.get_next_msg_id()),
                    in_reply_to: msg.body.msg_id,
                    payload: BroadcastPayload::ReadOk {
                        messages: state.messages.clone(),
                    },
                },
            })?,
            BroadcastPayload::ReadOk { .. } => {
                panic!("ReadOk message shouldn't be received by a node")
            }
        };
    }
    Ok(())
}
