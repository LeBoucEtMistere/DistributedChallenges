# Chapter 2: The Echo challenge

## Explanation

You can find the challenge here: [https://fly.io/dist-sys/1/](https://fly.io/dist-sys/1/)

The goal of this challenge is to discover how to interact with Maelstrom and build a very basic echo server.

You should start by reading the challenge description to get a better sense of how Maelstrom work. Then, this tutorial will take you step-by-step through a Rust solution.

## Forewords: working with Maelstrom
In the challenge description, you'll note that the author refer to a Go library they provide to easily interact with the Maelstrom nodes and clients. This repo provides you with a similar Rust library, located in the `node_driver` folder. It handles the initialization of your node, the communications over STDIN and STDOUT with Maelstrom, and the serialization and deserialization of the Messages.

We will explore this library as users in the tutorial, you can find the public documentation for this lib at [https://distributed-challenges.vercel.app/](https://distributed-challenges.vercel.app/). If you are getting confident with your Rust skills, I recommend taking some time later to explore this lib and see how it's coded.

## Walkthrough

### Discovering our first `struct` and `enum`

Reading through the challenge guide, we see our node will receive `echo` messages that look something like that:
```json
{
  "src": "c1",
  "dest": "n1",
  "body": {
    "type": "echo",
    "msg_id": 1,
    "echo": "Please echo 35"
  }
}
```
(If you want more details on the protocol Maelstrom uses to transmit messages, see [this](https://github.com/jepsen-io/maelstrom/blob/main/doc/protocol.md))

As explained above, the `node_driver` library provides you everything you need to model such messages. Therefore, before writing any Rust, we will start by reading the documentation of this library, and more particularly the documentation of the [Message struct](https://distributed-challenges.vercel.app/node_driver/struct.Message.html).

This is our first encounter with a `struct`, the concept Rust uses to define aggregates of data. This is in a way very similar to C structs as you define your data and the behaviors to operate on it separately. Our data structure definition looks like this:
```rust,ignore
pub struct Message<P> {
    pub src: String,
    pub dst: String,
    pub body: Body<P>,
}
```

The `pub` keyword at the struct level denotes that this is a public structure exposed by the library to its callers, while the `pub` keywords before each field of the struct denotes these fields are accessible from outside of the struct, this can be though of as visibility modifiers. Of course, the documentation only shows public fields and structs since they are the only ones we can interact with.

Each field is described as `field_name: field_type`.

We see two basic fields `src` and `dst`, of type String.

The last field type has a generic annotation (the `<P>` syntax, C++ users should feel pretty much at home here). It means that the `Message` struct is generic over a type, noted `P`, and here this type `P` is used in the definition of the type of the field `body`: a `Body<P>`. Let's look at the documentation for the type `Body` to understand what's going on here.

The `Body` struct is documented like this:
```rust,ignore
pub struct Body<P> {
    pub msg_id: Option<usize>,
    pub in_reply_to: Option<usize>,
    pub payload: P,
}
```

There are two new things there: we see two fields that are of type `Option<usize>`, and the last field is of the type `P`, our generic type parameter. Let's explain each.

`Option<T>` is another generic struct, in fact it comes from the Rust standard lib and is a staple of any Rust program, you can see the doc for this struct by clicking on it in the doc of the `Body` struct. This is actually an `enum` and not a `struct`, defined as:
```rust,ignore
pub enum Option<T> {
    None,
    Some(T),
}
```
(note that all enum fields have the visibility of the enum so no need to add `pub` to each variant here)

So far this definition looks pretty similar to a classic Python enum, except, the second variant `Some` actually contains data, of type `T` (the generic parameter of the enum). This is what we call a `sum type` or an `algebraic type`: an enum that can contain different data types based on the variant. In our case, we use an `Option<usize>` which means that the data contained in the `Some` variant will be of type `usize` (a `usize` is an unsigned integer of the size of a pointer on your system, most likely 64 bits on a 64 bits system), while the `None` variant contains nothing.

Algebraic types are one of the killer features of Rust, for more details, refer to the [official rust book](https://doc.rust-lang.org/book/ch06-00-enums.html). What's really important is that to get data out of your `Option` instance, you NEED to check if it's `None` or `Some` before, and the compiler will force you to do so. There is no way around this, and this prevents developers from accidentally forgetting that the result they deal with can be None. Python programmers will hopefully quickly realize how powerful this is, and how much safety it brings to the programs.

Let's go back at our `Body` struct and look at the `payload` field. We see it is of type `P`, the type over which our struct is generic. This means that we can stuff any type we want as a payload of our message, a `Message<f64>` will have a field body of type `Body<f64>` which in turns will have a field payload of type `f64`.

### Defining the messages

Armed with this knowledge, let's hop into our editor and start writing Rust to represent the `echo` messages. Open the file `distributed_challenges/src/bin/echo.rs`. You are faced with this:
```rust,ignore
fn main() -> anyhow::Result<()> {
    todo!()
}
```

This is a minimal Rust program, composed of a `main` function taking no arguments and returning a value of type `anyhow::Result<()>`. Let's not bother too much about this return type for now.
The corpus of the function is a single invocation of the `todo!()` macro. Any function postfixed with a `!` is what we call a `macro`. Let's not bother about this either, you can think of them as functions on steroids for now (If you want to know more, check out the [rust book](https://doc.rust-lang.org/book/ch19-06-macros.html)).

Let's define an enumeration to represent the payloads for the two messages our application will need to handle. Add this definition outside of the main function:

```rust,ignore
/// Defines the payload we want to send to clients in the echo challenge
enum EchoPayload {
    /// Used by clients to send an echo request
    Echo { echo: String },
    /// Used by nodes to respond to an echo request
    EchoOk { echo: String },
}
```

We have two variants, one for each message, and each of them contains a string field called `echo` that will represent the data we are asked to echo back. (The `///` defines docstring of the fields, while `//` are regular non-documenting comments)

Let's make our enum a little bit easier to work with by adding some useful behaviors to it: `Debug` and `Clone`. The first one will allow us to print the enum content to debug it, and the second will make it so we can clone instances of the enum if we need to.
In fact, these "behaviors" are what we call in Rust `traits`. A trait defines functions, and can be implemented on types. This is in a way the interfaces of Rust, although it is much more powerful than this.
The `Debug` and `Clone` traits are provided by the Rust standard library: [https://doc.rust-lang.org/std/clone/trait.Clone.html](https://doc.rust-lang.org/std/clone/trait.Clone.html) and [https://doc.rust-lang.org/std/fmt/trait.Debug.html](https://doc.rust-lang.org/std/fmt/trait.Debug.html). We could manually implement it for our type if we wanted to do fancy things, but the compiler is smart enough to infer basic implementations of these traits for us and all we have to do to get it is to add this line on top of our enum:
```rust,ignore
/// Defines the payload we want to send to clients in the echo challenge
#[derive(Debug, Clone)]
enum EchoPayload {
    /// Used by clients to send an echo request
    Echo { echo: String },
    /// Used by nodes to respond to an echo request
    EchoOk { echo: String },
}
```
This tells the Rust compiler to generate an implementation of both the `Clone` and `Debug` traits for the `EchoPayload` enum, this is what we call "deriving traits".

### Implementing the main loop of our server

Now that we have a payload enum, let's start writing the logic of our server. In the main, let's first initialize our Malestrom client:

```rust,ignore
use node_driver::Maelstrom;

// ... snip ...

fn main() -> anyhow::Result<()> {
    // init our node by getting its metadata and an output and input interface to communicate
    let (node_metadata, input, output) = Maelstrom::init()?;

    Ok(())
}
```

There's a bunch of new concepts here:
- we add a `use` statement to import the `Maelstrom` type from the `node_driver` lib.
- we define variables using the `let` keyword.
- we call the `init` class method of the `Maelstrom` struct using the `Maelstrom::init()` syntax.
- we use the `?` sigil to bubble up errors returned by the `init()` method to the output of the `main` function (you now see why we return a Result over the null type `()` as our main return type). If you look at the signature of the `init()` method in the documentation, you'll see it returns a `Result<(NodeMetadata, InputInterface, OutputInterface)>` and not just the tuple `(NodeMetadata, InputInterface, OutputInterface)`, a `Result` is also a sum type, very similar to `Option<T>`, which allows us to denote if a function errored out or if it returned a result.
- statements finish with a `;` while expressions don't, and if the last line of a function is an expression, it is returned automatically. You can also use the statement syntax `return Ok(());` but it is less idiomatic.


You'll see that if you remove the `?` sigil for instance, running `cargo build` or `cargo check` will give you a clear compiler error that the type don't match and that you need to ensure the result returned by the `init` method is not an error. This is what the `?` sigil does, it bubbles up any error and if there is none, it "unwrap" the data out of the Result type. I encourage you to try stuff and read compiler errors, they have been designed to be as informative as possible and explain properly the source of your issue and how to solve it.

Now that our node is initialized, we can add our main loop over the incoming messages:
```rust,ignore
// ... snip ...

fn main() -> anyhow::Result<()> {
    // init our node by getting its metadata and an output and input interface to communicate
    let (node_metadata, input, output) = Maelstrom::init()?;

    // main loop: for each message we receive through the input interface (with a payload of type EchoPayload)
    for msg in input.iter::<EchoPayload>() {

    }

    Ok(())
}
```

We see here a for loop with no body yet. We loop over the content of `input.iter::<EchoPayload>()`, let's decipher this. Our `input` object, of type `InputInterface`, has a method called `iter<P>()` which produces an iterator over items of type `Result<Message<P>>`. However, if we just wrote `for msg in input.iter()`, the compiler cannot infer which type of item it should be producing because it can't infer what `P` should be in this context. Therefore we use the "turbofish" syntax `::<>` (yes, this is a fish going fast) to disambiguate the call to `iter` and specify that `P` should be `EchoPayload` in our case.

However, if you try to compile this program with `cargo build`, you will be faced with errors:
```ignore
error[E0277]: the trait bound `for<'de> EchoPayload: serde::de::Deserialize<'de>` is not satisfied
  --> distributed_challenges/src/bin/echo.rs:17:29
   |
17 |     for msg in input.iter::<EchoPayload>() {}
   |                             ^^^^^^^^^^^ the trait `for<'de> serde::de::Deserialize<'de>` is not implemented for `EchoPayload`
   |
   = help: the following other types implement trait `serde::de::Deserialize<'de>`:
             &'a Path
             &'a [u8]
             &'a str
             ()
             (T0, T1)
             (T0, T1, T2)
             (T0, T1, T2, T3)
             (T0, T1, T2, T3, T4)
           and 129 others
   = note: required for `EchoPayload` to implement `serde::de::DeserializeOwned`
note: required by a bound in `InputInterface::iter`
  --> /Users/arthur.depasse/perso/DistributedChallenges/node_driver/src/lib.rs:98:12
   |
98 |         P: DeserializeOwned,
   |            ^^^^^^^^^^^^^^^^ required by this bound in `InputInterface::iter`

For more information about this error, try `rustc --explain E0277`.
error: could not compile `distributed_challenges` due to previous error
```

This looks rather cryptic at first, but you'll see it's actually a treasure trove of information to fix our error.

### Fulfilling trait bounds

Looking at the error message, we can read the following:

```ignore
the trait bound `for<'de> EchoPayload: serde::de::Deserialize<'de>` is not satisfied
...
note: required by a bound in `InputInterface::iter`
...
   |
98 |         P: DeserializeOwned,
   |            ^^^^^^^^^^^^^^^^ required by this bound in `InputInterface::iter`
```

This tells us that we are not fulfilling a constraint imposed by the `iter()` function. Let's look at its documentation:

This definition of the payload is not enough though. Let's have a look at the documentation of the `node_driver` lib and in particular, how to send a message with it: [https://distributed-challenges.vercel.app/node_driver/struct.OutputInterface.html#implementations](https://distributed-challenges.vercel.app/node_driver/struct.OutputInterface.html#implementations). In here we see this special syntax for the `send_msg` function:
```rust,ignore
pub fn iter<P>(&mut self) -> impl Iterator<Item = Result<Message<P>>> + '_
where
    P: DeserializeOwned,
```

It's admittedly a bit hairy because it is very explicit, but the interesting part for us is what comes after the `where` clause: `P: DeserializeOwned`. This is what we call a trait bound, the function enforces that any generic type `P` we use with it must at least implement the `DeserializeOwned` trait. This is a very important mechanisms that let's us do some kind of compile-time polymorphism: we can call iter with any type as long as we know it implement this interface. In this case, we need to enforce the message can be deserialized into an owned `Message<P>`.

Note: this `DeserializeOwned` trait is not coming from the standard Rust library but from another lib called `serde` that provides utils for serializing and deserializing data structures in a multitude of formats.

If we also look at [the function we use to send messages back](https://distributed-challenges.vercel.app/node_driver/struct.OutputInterface.html#method.send_msg), we see it has a bound as well, `P` must implement `Serialize`.

Therefore, let's implement both `serde::Serialize` and `serde:Deserialize` for our payload type (in our case implementing `Deserialize` is enough to get `DeserializeOwned`, for reasons we won't detail here).

We could implement these traits by hand but fortunately, `serde` comes with a feature that allows us to use the helpful `derive` statement to auto-implement them for struct that only contain data that also implement them (which is the case here, all default data types implement them).

```rust,ignore
use serde::{Serialize, Deserialize}
/// Defines the payload we want to send to clients in the echo challenge
#[derive(Debug, Clone, Serialize, Deserialize)]
enum EchoPayload {
    /// Used by clients to send an echo request
    Echo { echo: String },
    /// Used by nodes to respond to an echo request
    EchoOk { echo: String },
}
```

And now the error magically goes away as our Payload type meets the conditions required to deserialize it from a Maelstrom STDIN message.

We still have one error left though:
```ignore
error[E0596]: cannot borrow `input` as mutable, as it is not declared as mutable
  --> distributed_challenges/src/bin/echo.rs:18:16
   |
18 |     for msg in input.iter::<EchoPayload>() {}
   |                ^^^^^^^^^^^^^^^^^^^^^^^^^^^ cannot borrow as mutable
   |
help: consider changing this to be mutable
   |
15 |     let (node_metadata, mut input, output) = Maelstrom::init()?;
   |                         +++
```
This one can also be explained by looking at the signature of the `input` method:
```rust,ignore
pub fn iter<P>(&mut self) -> impl Iterator<Item = Result<Message<P>>> + '_
where
    P: DeserializeOwned,
```
Notice the `&mut` modifier in front of `self`? This means this method will act on a self that is a `mutable reference` to the instance it's called on. This means that `self` is not passed by value but by reference, and that this reference is an exclusive mutable reference that allows the user to modify the `InputInterface` instance represented by `self`.


Our error comes from the fact that variables in Rust are immutable by default. Because of this, `input` can only be dereferenced to `&self` when calling `iter`, and not `&mut self`.
We can simply fix that by adding the `mut` modifier to the definition of let to allow mutability. In fact, let's add it to `output` and `node_metadata` as well since we will also need them to be mutable in the future. Note that the error message suggests you a fix for this error.

```rust,ignore
    // init our node by getting its metadata and an output and input interface to communicate
    let (mut node_metadata, mut input, mut output) = Maelstrom::init()?;
```

There is one final touch we need to make to our `EchoPayload` before jumping to filling out the loop body. Indeed as stated in the documentation of the [body payload](https://distributed-challenges.vercel.app/node_driver/struct.Body.html#structfield.payload) and the [Maelstrom protocol](https://github.com/jepsen-io/maelstrom/blob/main/doc/protocol.md), we need to make sure our json payload we ultimately send contains a `type` field, which we haven't added to our enum variants.

There is a neat trick to do it automatically, based on the name of the variant, using `serde` annotations:
```rust,ignore
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
```

Note we added two `#[serde(...)]` annotations to our enum. The first one asks serde to use an internal tag field to hold the enum variant, and to name this field `type`, which serde will populate with the variant's name. The second tells serde to automatically rename all variants using snake_case, which will give us our `"type" = "echo"` and `"type" = "echo_ok"` in our json messages. For more details on handling of enums in `serde`, look at [https://serde.rs/enum-representations.html](https://serde.rs/enum-representations.html)


We end up with the following code at this point:
```rust,ignore
use node_driver::Maelstrom;
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
    for msg in input.iter::<EchoPayload>() {}

    Ok(())
}
```

### Finish implementing the main loop

It's now time to code the logic to react to the messages we receive.

First, we need to extract the message out of the `Result<Message<EchoPayload>>` returned by our iterator:

```rust,ignore
fn main() -> anyhow::Result<()> {
    // init our node by getting its metadata and an output and input interface to communicate
    let (mut node_metadata, mut input, mut output) = Maelstrom::init()?;

    // main loop: for each message we receive through the input interface (with a payload of type EchoPayload)
    for msg in input.iter::<EchoPayload>() {}
        // if there was an error getting this message, propagate it (with the ? sigil)
        let msg = msg?;
    Ok(())
}
```

Here we do a nice thing, we shadow the variable name `msg` by extracting the content or bubbling up any error using `?`. After this line, the type of `msg` is no longer `Result<Message<EchoPayload>>` but `Message<EchoPayload>` which simplifies our code.

Now, we will use a `match` statement to react differently to all the possible variants of the payload contained in the message:

```rust,ignore
    // main loop: for each message we receive through the input interface (with a payload of type EchoPayload)
    for msg in input.iter::<EchoPayload>() {
        // if there was an error getting this message, propagate it (with the ? sigil)
        let msg = msg?;
        // match on the type of payload within the message, these are variants of the EchoPayload enum
        match msg.body.payload {
            // if we get an Echo message, let's reply by crafting an EchoOk message and sending it through the output interface
            EchoPayload::Echo { echo } => todo!(),
            // we are not supposed to receive and EchoOk message, let's panic when it happens
            EchoPayload::EchoOk { .. } => panic!("EchoOk message shouldn't be received by a node"),
        };
    }
```

`match` statements are one of the killer features of Rust, they are really powerful, support pattern matching, and need to be exhaustive.

Here we match on `msg.body.payload` which is of type `EchoPayload`, each arm of the match statement if of the form `pattern => expression,` or `pattern => statement,`.
We use simple patterns here, one for each possible variant. In the first one, we use the pattern to deconstruct the `echo` field within the variant since we will need it to craft the response. In the second arm, we don't need it so we elide the deconstruction with the `..`.
Finally, the second match arm is not supposed to happen in normal operations, so when it does we simply invoke the `panic!()` macro that will safely exit the program with an error message.

Finally, let's complete the first match arm and send a response message:
```rust,ignore
// ...
match msg.body.payload {
    EchoPayload::Echo { echo } => output.send_msg(Message {
        src: node_metadata.node_id.clone(),
        dst: msg.src,
        body: Body {
            msg_id: Some(node_metadata.get_next_msg_id()),
            in_reply_to: msg.body.msg_id,
            payload: EchoPayload::EchoOk { echo },
        },
    })?,
    /// ...
```

This version is written concisely, but we could have made it a little clearer to new rustaceans by using a `{}` expression block here:
```rust,ignore
match msg.body.payload {
    EchoPayload::Echo { echo } => {
        let response = Message {
            src: node_metadata.node_id.clone(),
            dst: msg.src,
            body: Body {
                msg_id: Some(node_metadata.get_next_msg_id()),
                in_reply_to: msg.body.msg_id,
                payload: EchoPayload::EchoOk { echo },
            }
        };
        output.send_msg(response)?
    },
    /// ...
```

We see that we instantiate a `Message` with an `EchoOk` payload, obtain a new message id from our `NodeMetadata` instance, and use our output interface to send it.

You should end up with the following code:

```rust,ignore
{{#include ../../distributed_challenges_solution/src/bin/echo_solution.rs}}
```

### Testing our solution

It's now time to test our solution using Maelstrom:

```bash
cargo build
~/maelstrom/maelstrom test -w echo --bin target/debug/echo --node-count 1 --time-limit 10
```

If it all goes well, you should ultimately see this message:
```ignore
Everything looks good! ヽ(‘ー`)ノ
```

Congrats, you finished Challenge 1!
