use anyhow::Context;
use node_driver::{Body, Maelstrom, Message};
use serde::{Deserialize, Serialize};

/// Defines the payload we want to send to clients in the echo challenge
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
enum UniqueIdPayload {
    /// Used by clients to send a generate request
    Generate,
    /// Used by nodes to respond to a generate request, we use a string typed ID since we
    /// will return uuid-v4
    GenerateOk { id: String },
}

fn main() -> anyhow::Result<()> {
    // init our node by getting its metadata and an output and input interface to communicate
    let (mut node_metadata, mut input, mut output) = Maelstrom::init()?;
    // main loop: for each message we receive through the input interface (with a payload of type UniqueIdPayload)
    for msg in input.iter::<UniqueIdPayload>() {
        // if there was an error getting this message, propagate it (with the ? sigil)
        let msg = msg.context("While parsing unique_id message")?;
        // match on the type of payload within the message, these are variants of the UniqueIdPayload enum
        match msg.body.payload {
            // if we get a Generate message, let's reply by crafting an GenerateOk message and sending it through the output interface
            UniqueIdPayload::Generate => output.send_msg(Message {
                src: node_metadata.node_id.clone(),
                dst: msg.src,
                body: Body {
                    msg_id: Some(node_metadata.get_next_msg_id()),
                    in_reply_to: msg.body.msg_id,
                    payload: UniqueIdPayload::GenerateOk {
                        // let's generate a uuid v4 using the uuid crate
                        id: uuid::Uuid::new_v4().to_string(),
                    },
                },
            })?,
            // we are not supposed to receive a GenerateOk message, let's panic when it happens
            UniqueIdPayload::GenerateOk { .. } => {
                panic!("GenerateOk message shouldn't be received by a node")
            }
        };
    }
    Ok(())
}
