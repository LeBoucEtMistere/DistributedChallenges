# Chapter 4b: Multi node broadcast

You can find the challenge here: [https://fly.io/dist-sys/3b/](https://fly.io/dist-sys/3b/)

This challenge is where the real fun begins! Let's make our nodes communicate with each others.

## A bit of theory

### Gossip

To do so, we will use one of the staples of distributed systems: Gossip. From [the wikipedia page](https://en.wikipedia.org/wiki/Gossip_protocol#:~:text=A%20gossip%20protocol%20or%20epidemic,all%20members%20of%20a%20group.): 
> A gossip protocol or epidemic protocol is a procedure or process of computer peer-to-peer communication that is based on the way epidemics spread. Some distributed systems use peer-to-peer gossip to ensure that data is disseminated to all members of a group.

By designing a broadcast system, the challenge encourages us to develop a leaderless architecture: we will develop a peer-to-peer architecture, which can be be very similar to what is used in Cassandra for instance. Every node is doing the same work and if you send a message to node A and then ask node B to get this same message, it should eventually be able to provide it to you. The word eventually is important, these systems are generally "eventually consistent" because it takes time to propagate state updates to all nodes. Essentially, this means that given enough time (a reasonable amount of it) after the last state update, all nodes should have the same state.

For now we will implement an inefficient but simple version of gossip: whenever each node decides to gossip, it will send all the data it knows of to all other nodes. We will improve this in the next parts of this challenge.

### Scheduling gossip with an actors model

Next important thing is to decide when we want to gossip. We could do it after every message we receive but this seems terribly wasteful. Since we are ok with eventual consistency, let's gossip on a timer, every 100ms for instance.

This means that we will need to change our main loop so that every so often, we pause message-listening to gossip what we know to other nodes in our topology. There are multiple way to implement this interleaved scheduling of two I/O tasks (waiting on a timer and reading new messages from stdin): we could leverage async Rust to do so with high performances, but this is far too complex for today's workshop, so instead we will use a simple multi-threaded approach by building an actors model.

An actor model is a model in which each actor (generally a thread) is responsible for one part of the work, and each actor can only communicate with other actors through the use of messages. Actors models are very powerful and can make the writing of multi-threaded code much easier and less risky, since they essentially remove the need for lock-based synchronization. However, they also can often results in bottlenecks so they are not always the best choice. However for our use case it will work well.

## Walkthrough

In this part we will need to learn how to do new stuff in Rust: writing multi-threaded code in particular so we will start from the content of the solution of the previous part and elaborate on it progressively while explaining everything.

We will have 3 different actors (so 2 additional threads to the main thread):
- **actor 1:** run a timer and periodically ask for gossip to happen
- **actor 2:** read messages from input
- **actor 3:** consume the output of the previous two and run the main loop (i.e. send messages).

Let's start by representing the messages our actors will send each others (we will call them `Event` to avoid confusion with the `Message` struct):

```rust,ignore
/// This defines the possible events on which our main loop can react, within our actor system
enum Event {
    /// this event means there is no more input messages to read from Maelstrom
    Eof,
    /// this event means it's time to do some gossip
    TimeToGossip,
    /// this event means we have received a message
    MessageReceived(Message<BroadcastPayload>),
}
```
As you can see, we use an enum with 3 variants. The first one will be used to mark the end of the input and properly shut down other threads.

The second one will be emitted by our first actor at a periodic interval

The third one encapsulates a Message and is sent by the actor that consumes from stdin.

Essentially, we will have this architecture:
```ignore
         ┌─────────┐
         │         │
timer ──►│ actor 1 ├─┐
         │         │ │                ┌─────────┐
         └─────────┘ │ Channel<Event> │         │
                     ├───────────────►│ actor 3 ├──► stdout
         ┌─────────┐ │                │         │
         │         │ │                └─────────┘
stdin ──►│ actor 2 ├─┘
         │         │
         └─────────┘
```

As you can see, we will use a channel to send and receive messages, we need a channel that allows multiple producers and single receiver (an MPSC channel in Rust lingua, Multiple Producers, Single Consumer). To declare such a channel, we need to write this code:

```rust,ignore
// we will use an actor channel to handle scheduling of both gossiping and reading and
// responding to messages.
// create a channel that we will use to make our actors communicate
let (tx, rx) = std::sync::mpsc::channel::<Event>();
```

We can again observe the turbofish syntax used to specify which type of things we want to send through our channel. Sum types are such a natural fit to describe a message going through a channel: you have a single type, that can describe several variants, each carrying their own payload.
When calling this `channel` function we get its two handles, `tx` (for transmit) to send events and `rx` (for receive) to receive events.

### Actor 1: sending Gossip events

Now we only need to write our actors. Let's start with actor 1, a thread that will loop and periodically send `Event::TimeToGossip` events in the channel. The code to do so looks like this

```rust,ignore
use std::time::Duration;
// ...

// clone the tx handle so that we keep one for the actor 2 as well
let tx_clone = tx.clone();
// spawn a thread generating periodic gossip events, our first actor
let gh = std::thread::spawn(move || loop {
    if tx_clone.send(Event::TimeToGossip).is_err() {
        // other side hung up, let's finish the loop
        break;
    };
    std::thread::sleep(Duration::from_millis(250));
});
```

We can see here an example of a "closure", i.e. an anonymous function that can capture data from its invocation context. This is very similar to python lambdas. The syntax to declare a closure is 
```ignore
[optional: move] |[args...]| [expression]
```
Let's start with a simple example:
```rust,ignore
let a: u8 = 23;
let closure = |b: u8| a + b;

println!("The result is {}", closure(7));
```
This will print `30`. This closure "captures" the a variable from its outer scope, and sums it with its parameter b. In the context of the closure, `a` would be a `&u8` as closures capture variables by reference by default. If we want to capture outer variables by moving them into the closure scope instead of taking references, we prefix the closure syntax with the `move` keyword. In that case, `a` would be a `u8` within the closure, and after invoking it, you could no longer use `a` in the outer scope as it would have been moved away (consumed by the closure if you want).

Let's go back to our original closure that defines our thread:
```rust,ignore
move || loop {
    if tx_clone.send(Event::TimeToGossip).is_err() {
        // other side hung up, let's finish the loop
        break;
    };
    std::thread::sleep(Duration::from_millis(250));
}
```

We use `move` because we cannot take references and send them to another thread (this is not memory safe and rust won't let you do it). We capture the `tx_clone` handle (by value because of the `move` modifier), and in an infinite loop (the `loop` keyword is syntactic sugar for `while true`), we send a `Event::TimeToGossip` event in the channel and then sleep the thread for 250ms.
If the send call returns an error, we break out of the loop and therefore finish the thread. This will happen when the other side of the channel hangs up and we will use this mechanism to properly finish our threads and shutdown our application.

### Actor 2: reading stdin for incoming messages
Let's show the code for the second actor and then comment it:

```rust,ignore
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
```

There is not much more going here but I'll detail a few points still:
- we don't clone `tx` once more, we directly captures the original `tx` into this thread since we won't need it anymore for other actors.
- once we have read all the messages from stdin, we send the special `Event::Eof` event in the channel to tell actor 3 that it's time to do shutdown. Once actor 3 shuts down, it will automatically signal actor 1 to shutdown as well by deleting the `rx` end of the channel.
- We need to acquire our mutable input interface within the thread scope. This is because an input interface is actually a mutex over stdin (to prevent several threads from accessing it simultaneously which would read garbage data), and a mutex cannot be sent safely to another thread. To make it work, we also need to change our Maelstrom init call to immediately drop the input handle we get from it and let this new thread acquire the lock itself. So you will need to change your init call to something like this: `let (mut node_metadata, _, mut output) = Maelstrom::init()?;` where `_` is a special sigil which prevents the binding from happening and that automatically drops the returned value.

at this point, your main function should look something like this:
```rust,ignore
// ... imports ...

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

    // we now need to write the third actor in the main thread
    todo!()
}
```

### Actor 3: consuming events and acting on it

Finally we will write our third actor that will simply consume events from the channel and react on them. On a gossip event, we will send messages to other nodes to let them know of the data we know. On a message event, we will respond to this message, possibly using the data we have in our state. Finally, on an eof event, we will simply stop the program properly.

So let's start coding this by looping over the channel content and matching on the event type:
```rust,ignore
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
            todo!()
        }
        Event::MessageReceived(msg) => {
            // match on the type of payload within the message, these are variants of the BroadcastPayload enum
            match &msg.body.payload {
                BroadcastPayload::Gossip { known } => {
                    // we received a gossip message from another node, let's update our known data
                    todo!()
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
ih.join().unwrap();
gh.join().unwrap();
```

In case of an Eof, we are just breaking out of the outer loop and then calling `join()` on both our thread handles to wait until they have complete and properly exit our program.

In case of a message received, we do the same as we did in the previous part, except now this time we support a new payload type: `BroadcastPayload::Gossip`. This is a one shot message that don't expect a response, and that looks like this:
```rust,ignore
/// Defines the payload we want to send to clients in the broadcast challenge
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
enum BroadcastPayload {
    // ... snip ...
    // we will use this message to communicate gossip in-between nodes of the cluster
    Gossip {
        known: HashSet<usize>,
    },
}
```

Now we just need to implement how to react on such a gossip message received, and how to react to a `TimeToGossip` event.

For the latter we will just send a `Gossip` message with the messages we have stored in our state, and for the former we will update our state with the unknown messages we received:

```rust,ignore
// ...
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
// ...
```

```rust,ignore
// ...
match &msg.body.payload {
    BroadcastPayload::Gossip { known } => {
        // we received a gossip message from another node, let's update our known data
        state.messages = state.messages.union(known).copied().collect();
    }
// ...
}
```

And that's all for this challenge, we implemented a complete actor model based on multithreading to act on several types of events coming from various sources in our program.

### Testing our code
It's now time to build and test our code to verify if we succeeded. First let's run `cargo build` to build a debug binary of our program. This should generate a new binary: `target/debug/broadcast_2`.

Let's now invoke Maelstrom on it to see if we solved the challenge:
```bash
~/maelstrom/maelstrom test -w broadcast --bin ./target/debug/broadcast_2 --node-count 5 --time-limit 20 --rate 10
```

If everything goes well, you should see
```ignore
Everything looks good! ヽ(‘ー`)ノ
```
Congrats! In the next parts, we will work at making our gossip smarter and fault tolerant.
