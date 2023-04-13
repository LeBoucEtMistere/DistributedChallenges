# Chapter 4: The Unique Id challenge
## Explanation

You can find the challenge here: [https://fly.io/dist-sys/1/](https://fly.io/dist-sys/1/)

The goal of this challenge is to generate unique ids across several nodes. There are two approaches to this solution:
- let the nodes communicate between them to sync (complex)
- generate ids with enough entropy to avoid collisions between two generation no matter the node (easy)

We will follow the second approach and rely on [UUIDs](https://en.wikipedia.org/wiki/Universally_unique_identifier) generation algorithms, and more specifically UUIDs V4.

## Walkthrough

The code will be very similar to the Echo challenge code, so feel free to copy and paste and try it by yourself if you want, the only new concepts will be around importing a library (a crate in rust lingua) to our project, but other than this, you already have all the knowledge to do it by yourself.

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

As in the previous part, this paylaod enum is made to be inserted in the payload field of the `Message` type provided by the `node_driver` lib.
