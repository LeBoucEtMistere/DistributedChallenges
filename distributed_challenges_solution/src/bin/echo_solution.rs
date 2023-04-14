use node_driver::{Body, Maelstrom, Message};
use serde::{Deserialize, Serialize};

/// Defines the payload we want to send to clients in the echo challenge
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
enum EchoPayload {
    /// Used by clients to send an echo request
    Echo { echo: String },
    /// Used by nodes to respond to an echo request
    EchoOk { echo: String },
}

fn main() -> anyhow::Result<()> {
    // init our node by getting its metadata and an output and input interface to communicate
    let (mut node_metadata, mut input, mut output) = Maelstrom::init()?;
    // main loop: for each message we receive through the input interface (with a payload of type EchoPayload)
    for msg in input.iter::<EchoPayload>() {
        // if there was an error getting this message, propagate it (with the ? sigil)
        let msg = msg?;
        // match on the type of payload within the message, these are variants of the EchoPayload enum
        match msg.body.payload {
            // if we get an Echo message, let's reply by crafting an EchoOk message and sending it through the output interface
            EchoPayload::Echo { echo } => output.send_msg(Message {
                src: node_metadata.node_id.clone(),
                dst: msg.src,
                body: Body {
                    msg_id: Some(node_metadata.get_next_msg_id()),
                    in_reply_to: msg.body.msg_id,
                    payload: EchoPayload::EchoOk { echo },
                },
            })?,
            // we are not supposed to receive and EchoOk message, let's panic when it happens
            EchoPayload::EchoOk { .. } => panic!("EchoOk message shouldn't be received by a node"),
        };
    }
    Ok(())
}
