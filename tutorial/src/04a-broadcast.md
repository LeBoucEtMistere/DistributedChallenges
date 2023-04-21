# Chapter 4a: Single node broadcast

You can find the challenge here: [https://fly.io/dist-sys/3a/](https://fly.io/dist-sys/3a/)

In this first part of the broadcast challenge, we will make a system with a single node, so that we don't have to worry about inter-node communication and we can just focus on the interfaces to communicate with clients.

This first part is very similar to the previous exercises and doesn't involve anything new so we won't detail the code here. However we will discuss some interesting points, and if you need you can always check the solution code in the repo for a working implementation.

The test command should be 
```bash
~/maelstrom/maelstrom test -w broadcast --bin target/debug/broadcast_1 --node-count 1 --time-limit 20 --rate 10
```


## The messages
Once again we are presented with a list of messages our application needs to know about. `read` and `broadcast` are pretty straightforward, but `topology` is a bit different. 

This `topology` message is sent once to each node in the network at the start by maelstrom and suggests a topology for each node to use. A topology is a graph describing which nodes connect with which other nodes. In the real world, this would be dictated by communication latencies, network hops, or other parameters. In fact, there exists a bunch of strategies to define the optimal topology to ensure the best performances. In the context of Maelstrom, every node can speak with every node and you are not constrained by the topology that your node receive, you can perfectly ignore it or change it. Today we will actually use it and each node will act as if it only knows about the nodes in the topology it receives.

Note that you need to acknowledge the topology message as well.

In this first part, since there is only one node in the network, we won't use the topology, but to prepare for part 2, you will probably need to store it somewhere so that the node can reference it during the rest of its lifespan.

Since you will also need to keep track of all the messages the node knows about, I suggest you implement a structure dedicated to storing the state of the node.
