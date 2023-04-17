# Chapter 3: The Unique Id challenge
## Explanation

You can find the challenge here: [https://fly.io/dist-sys/2/](https://fly.io/dist-sys/2/)

The goal of this challenge is to generate unique ids across several nodes. There are two approaches to this solution:
- let the nodes communicate between them to sync (complex)
- generate ids with enough entropy to avoid collisions between two generation no matter the node (easy)

We will follow the second approach and rely on [UUIDs](https://en.wikipedia.org/wiki/Universally_unique_identifier) generation algorithms, and more specifically UUIDs V4.

## Walkthrough

The code will be very similar to the Echo challenge code, so feel free to copy and paste and try it by yourself if you want, the only new concepts will be around importing a library (a crate in rust lingua) in our project, but other than this, you already have all the knowledge to reach the last section of this chapter by yourself.

### Defining the message payloads

Let's start by defining the messages we want to receive and send, based on the description provided by the challenge:
We have a Generate request with the following body
```json
{
  "type": "generate"
}
```
and a GenerateOk response with the following body:
```json
{
  "type": "generate_ok",
  "id": "123"
}
```
(note the id can be of any type but we will stick with a string here since that's what UUIDs use)

Therefore, we will write the following enum to describe the payloads our application can deal with:
```rust,ignore
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
```

As in the previous part, you can note the various annotations on the enum to derive the common basic traits `Debug` and `Clone`, and the traits related to ser/de `Serialize` and `Deserialize`. These generate all the base scaffolding code under the hood to enable this enum variants to be easily displayed, cloned, serialized and deserialized (to json in particular which is what we care for here). There also are `serde` relative annotations to internally tag the enum variants with a type field, and rename all variants to a snake_case format. This basically gives us the required `type = ` field in the json payload.

We have two variants, Generate representing generate requests, which doesn't contain any data, and GenerateOk representing a response to this request, that contains a field named `id` of type String.

As in the previous part, this payload enum is made to be inserted in the payload field of the `Message` type provided by the `node_driver` lib.


### Writing the main loop

Now that we have our definition of the message payloads, let's start writing the main loop logic.
As in the previous part, we need to start by initializing our Maelstrom node (don't hesitate to look again at the documentation of the library at [https://distributed-challenges.vercel.app/](https://distributed-challenges.vercel.app/)):

```rust,ignore
fn main() -> anyhow::Result<()> {
    // init our node by getting its metadata and an output and input interface to communicate
    let (mut node_metadata, mut input, mut output) = Maelstrom::init()?;
}
```
This gets us our input and output interfaces and the node metadata (that we also won't use much in this challenge).

Let's start listening to incoming messages by looping on a blocking iterator on the messages received:

```rust,ignore
fn main() -> anyhow::Result<()> {
    // init our node by getting its metadata and an output and input interface to communicate
    let (mut node_metadata, mut input, mut output) = Maelstrom::init()?;
    // main loop: for each message we receive through the input interface (with a payload of type UniqueIdPayload)
    for msg in input.iter::<UniqueIdPayload>() {
        // if there was an error getting this message, propagate it (with the ? sigil)
        let msg = msg?;

        todo!()
    }
}
```
This time, we specify we want to receive messages with a payload of the type `UniqueIdPayload` that we just defined. We still have a line to propagate any error coming from the parsing/reception of the message.

We can now match on this message and define how we want to react to each variants. We will again use a `match` statement to do so:

```rust,ignore
    // match on the type of payload within the message, these are variants of the UniqueIdPayload enum
    match msg.body.payload {
        // if we get a Generate message, let's reply by crafting an GenerateOk message and sending it through the output interface
        UniqueIdPayload::Generate => todo!(),
        // we are not supposed to receive a GenerateOk message, let's panic when it happens
        UniqueIdPayload::GenerateOk { .. } => {
            panic!("GenerateOk message shouldn't be received by a node")
        }
    };
```
We already know we are not supposed to receive any `GenerateOk` message, so let's panic with a clear error message when this happens. Keep in mind a panic is a controlled exit of the program, that should be used when we are faced with an unrecoverable error. One could argue we should just ignore the invalid message but panicking on it could help us catch errors if we were to implement our own clients.

Let's no complete the match arm associated with the `Generate` message. We know we want to build a `GenerateOk` message with a new UUID and send it through our output interface:

```rust,ignore
    // match on the type of payload within the message, these are variants of the UniqueIdPayload enum
    match msg.body.payload {
        // if we get a Generate message, let's reply by crafting an GenerateOk message and sending it through the output interface
        UniqueIdPayload::Generate => output.send_msg(msg.to_response(
                Some(node_metadata.get_next_msg_id()), // obtain the next message id
                UniqueIdPayload::GenerateOk {
                    // let's generate a uuid v4 using the uuid crate
                    id: todo!(),
                },
            ))?,
        // we are not supposed to receive a GenerateOk message, let's panic when it happens
        UniqueIdPayload::GenerateOk { .. } => {
            panic!("GenerateOk message shouldn't be received by a node")
        }
    };
```

### Generating a UUID
The last missing part is now to generate an UUID. To do so, we will add a new dependency to our project, what is called a `crate` in Rust lingua: a small unit of shared code. Rust has first class support for easily managing dependencies of your project, through the `cargo` CLI, using a central crate registry: [crates.io](https://crates.io)

The dependency we want to use is [uuid](https://crates.io/crates/uuid). On this page you can see several important information on the crate:
- its version
- some explanations and examples of what it does
- how to install it
- links to the documentation of this crate and its repository

Let's add this crate to our project:
```bash
cargo add uuid -p distributed_challenges --features v4
```
This command tells cargo to add the crate `uuid` to our project, in the package `distributed_challenges`, with the feature `v4`. We need to specify which package to install it into since we are working in a cargo workspace, i.e. a collection of several rust packages (c.f. [the official documentation for more info on workspaces](https://doc.rust-lang.org/book/ch14-03-cargo-workspaces.html)). The `--features` flag lets us specify which features we want to use. Features are functionalities of the crate that are not shipped by default (often to avoid bloating your binary if you don't explicitly need some things), and that we can opt-in. Here we need to opt-in the `v4` feature to have the part of the lib responsible for generating UUIDs V4.

Note that after running this command, you should see this modification reflected in the `Cargo.toml` file at the root of the package `distributed_challenges`. Feel free to have a look at the other dependencies we reference there, some are inherited from global workspace dependencies, some are path-based dependencies, etc..

Now that we have this new dependency, let's use it. A good idea is always to start by looking at its documentation so let's visit [https://docs.rs/uuid/latest/uuid/](https://docs.rs/uuid/latest/uuid/).
Here we can see the minimal snippet of code to generate a `Uuid` object:
```rust,ignore
use uuid::Uuid;

let id = Uuid::new_v4();
```
Remember we ultimately need a String to put in our message, so let's navigate to the doc page of the `Uuid` structure to see if there are any methods or traits it implements that lets us convert it to a String: [https://docs.rs/uuid/latest/uuid/struct.Uuid.html#impl-ToString-for-Uuid](https://docs.rs/uuid/latest/uuid/struct.Uuid.html#impl-ToString-for-Uuid). We observe that the type implements the `ToString` trait that exposes the `to_string` method. Let's use it to complete our code:

```rust,ignore
// match on the type of payload within the message, these are variants of the UniqueIdPayload enum
    match msg.body.payload {
        // if we get a Generate message, let's reply by crafting an GenerateOk message and sending it through the output interface
        UniqueIdPayload::Generate => output.send_msg(msg.to_response(
                Some(node_metadata.get_next_msg_id()), // obtain the next message id
                UniqueIdPayload::GenerateOk {
                    // let's generate a uuid v4 using the uuid crate
                    id: uuid::Uuid::new_v4().to_string(),
                },
            ))?,
        // we are not supposed to receive a GenerateOk message, let's panic when it happens
        UniqueIdPayload::GenerateOk { .. } => {
            panic!("GenerateOk message shouldn't be received by a node")
        }
    };
```
Note that this time, instead of creating our `Message` instance manually, we used the utility method `to_response` implement on the `Message` type, which makes our code more concise.

### Testing our code
It's now time to build and test our code to verify if we succeeded. First let's run `cargo build` to build a debug binary of our program (if you run this from the root of the workspace, it will rebuild all packages that need it, you can always only rebuild the `unique_id` one using the `-p` flag of `cargo build`). This should generate a new binary: `target/debug/unique_id`.

Let's now invoke Maelstrom on it to see if we solved the challenge:
```bash
~/maelstrom/maelstrom test -w unique-ids --bin ./target/debug/unique_id --time-limit 30 --rate 1000 --node-count 3 --availability total --nemesis partition
```

If everything goes well, you should see
```ignore
Everything looks good! ヽ(‘ー`)ノ
```
Congrats!

