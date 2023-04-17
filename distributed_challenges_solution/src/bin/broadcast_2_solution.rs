use std::{
    collections::{HashMap, HashSet},
    time::Duration,
};

use anyhow::Context;
use node_driver::{Body, InputInterface, Maelstrom, Message};
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
    // we will use this message to communicate gossip in-between nodes of the cluster
    Gossip {
        known: HashSet<usize>,
    },
}

/// This struct holds the internal state of our node
struct State {
    pub messages: HashSet<usize>,
    /// topology is optional since we don't have it when we construct State in the first place
    pub topology: Option<HashMap<String, Vec<String>>>,
}

/// This defines the possible events on which our main loop can react, within our actor system
enum Event {
    /// this event means there is no more input messages to read from Maelstrom
    Eof,
    /// this event means it's time to do some gossip
    TimeToGossip,
    /// this event means we have received a message
    MessageReceived(Message<BroadcastPayload>),
}

fn main() -> anyhow::Result<()> {
    // init our node by getting its metadata and an output and input interface to communicate
    // here we drop the input interface as soon as we get it to release the lock before opening a
    // new one in a separate thread.
    let (mut node_metadata, _, mut output) = Maelstrom::init()?;

    // init the state
    let mut state = State {
        messages: HashSet::new(),
        topology: None,
    };

    // we will use an actor channel to handle scheduling of both gossiping and reading and
    // responding to messages.
    // create a channel that we will use to make our actors communicate
    let (tx, rx) = std::sync::mpsc::channel::<Event>();
    let tx_clone = tx.clone();

    // spawn a thread generating periodic gossip events, our first actor
    let gh = std::thread::spawn(move || loop {
        if tx_clone.send(Event::TimeToGossip).is_err() {
            // other side hung up, let's finish the loop
            break;
        };
        std::thread::sleep(Duration::from_millis(250));
    });

    // spawn a thread forwarding input into the channel, our second actor
    let ih = std::thread::spawn(move || {
        // get a new input interface, this can hang if another one already exists somewhere...
        let mut input = InputInterface::default();

        for msg in input.iter::<BroadcastPayload>() {
            let msg = msg.expect("Should be able to get message from stdin");
            if tx.send(Event::MessageReceived(msg)).is_err() {
                break;
            };
        }
        // no more messages, send EOF for proper shutdown
        tx.send(Event::Eof).unwrap();
    });

    // main loop: for each event we receive through the channel (our last actor)
    for event in rx {
        match event {
            // match on the type of event received
            Event::Eof => {
                // rx is automatically dropped once we get out of the loop because we implicitly called
                // into_iter() on it to buils the loop, which consumes self.
                break;
            }
            Event::TimeToGossip => {
                // it's time to gossip, let's send messages to all nodes within our reach
                if let Some(topology) = state.topology.as_ref() {
                    for n in topology.get(&node_metadata.node_id).context(format!(
                        "Node {} should appear in the topology",
                        node_metadata.node_id
                    ))? {
                        // for now we send the full list of messages we know, which is suboptimal
                        output.send_msg(Message {
                            src: node_metadata.node_id.clone(),
                            dst: n.clone(),
                            body: Body {
                                msg_id: None,
                                in_reply_to: None,
                                payload: BroadcastPayload::Gossip {
                                    known: state.messages.clone(),
                                },
                            },
                        })?;
                    }
                }
                // if we don't have the topology yet, let's skip gossiping for now.
            }
            Event::MessageReceived(msg) => {
                // match on the type of payload within the message, these are variants of the BroadcastPayload enum
                match &msg.body.payload {
                    BroadcastPayload::Gossip { known } => {
                        // we received a gossip message from another node, let's update our known data
                        state.messages = state.messages.union(known).copied().collect();
                    }
                    BroadcastPayload::Topology { topology } => {
                        state.topology = Some(topology.clone());
                        output.send_msg(msg.to_response(
                            Some(node_metadata.get_next_msg_id()),
                            BroadcastPayload::TopologyOk,
                        ))?
                    }
                    // we are not supposed to receive a TopologyOk message, let's panic when it happens
                    BroadcastPayload::TopologyOk => {
                        panic!("TopologyOk message shouldn't be received by a node")
                    }
                    BroadcastPayload::Broadcast { message } => {
                        state.messages.insert(*message);
                        output.send_msg(msg.to_response(
                            Some(node_metadata.get_next_msg_id()),
                            BroadcastPayload::BroadcastOk,
                        ))?
                    }
                    // we are not supposed to receive a BroadcastOk message, let's panic when it happens
                    BroadcastPayload::BroadcastOk { .. } => {
                        panic!("BroadcastOk message shouldn't be received by a node")
                    }
                    BroadcastPayload::Read => output.send_msg(msg.to_response(
                        Some(node_metadata.get_next_msg_id()),
                        BroadcastPayload::ReadOk {
                            messages: state.messages.clone(),
                        },
                    ))?,
                    // we are not supposed to receive a ReadOk message, let's panic when it happens
                    BroadcastPayload::ReadOk { .. } => {
                        panic!("ReadOk message shouldn't be received by a node")
                    }
                }
            }
        };
    }

    // let's join on both threads for proper exit
    ih.join().expect("input thread got poisoned");
    gh.join().expect("gossip thread got poisoned");
    Ok(())
}
