# 🎭 Workflow CLI: The Most Overengineered YAML Runner in Existence

Welcome to the **Workflow CLI** - a command-line tool that takes the simple concept of "run a bash command from a YAML file" and turns it into a distributed, actor-based, event-sourced, internationalized, fault-tolerant masterpiece of unnecessary complexity! 🚀

## What Does This Thing Actually Do?

At its core, this application does something absolutely revolutionary: it reads YAML files and... **copies commands to your clipboard**. That's it. No execution. No running. Just good old-fashioned Ctrl+V material.

I know, I know - we could have just used `cat file.yaml | grep command | pbcopy`, but where's the fun in that? Instead, why not build a distributed, fault-tolerant, event-sourced clipboard manager with actors, journals, and internationalized error messages?

## Features That Nobody Asked For

- 🎪 **Interactive Workflow Selection**: Choose your YAML file through a beautiful CLI menu (because `ls *.yaml` is for peasants)
- 🎭 **Actor Supervision Trees**: Guardian actors watching WorkflowManager actors watching CommandProcessor actors (it's turtles all the way down)
- 📚 **Event Journaling**: Every clipboard copy is persisted as an event (because what if you need to replay that `echo "hello"` copy operation?)
- 🌍 **Multi-language Support**: Clipboard errors in English AND Spanish (porque los errores de portapapeles son internacionales)
- 🔄 **Command Chaining**: Commands can schedule other clipboard operations (because clipboard recursion is fun)
- 📦 **Pluggable Storage**: Swap between in-memory and... well, just in-memory for now (but the abstraction is there!)

## Usage

```bash
# The simple way (boring)
cat my-commands.yaml | grep command | pbcopy

# The ENTERPRISE way (exciting!)
workflow
# Now paste with Cmd+V like a true enterprise developer!
```

## Architecture: A Study in Overengineering

```
Guardian Actor
├── WorkflowManager Actor
│   ├── CommandProcessor Actor (Session 1)
│   │   ├── Engine (Pure Business Logic™)
│   │   ├── Journal (Pluggable Persistence™)
│   │   └── EventStore (Because Events Are Life™)
│   └── CommandProcessor Actor (Session N)
└── Supervision Strategy (Because Actors Need Babysitting™)
```

Each YAML file selection spawns its own CommandProcessor actor with its own Engine instance and Journal for maximum isolation. Because you never know when copying `ls -la` to clipboard might crash the entire system.

## Still Not Overengineered Enough?

Don't worry! There are still plenty of opportunities to add more unnecessary complexity:

- [ ] **Distributed Mode**: Why run on one machine when you can have a cluster?
- [ ] **GraphQL API**: Because REST is so 2010
- [ ] **Blockchain Integration**: Probably?
- [ ] **Machine Learning**: AI-powered YAML file recommendations, because the hype train might end soon
- [ ] **Service Mesh**: Because even bash commands need Istio
- [ ] **Event Streaming**: Kafka for command events (obviously)

## Installation

If you don't know how to install a rust program, you should probably go check yourself.

## Configuration

Code is.. self explanatory?

## Contributing

Never heard of her

## License

This project is licensed under the "Why Did I Do This To Myself" license.

---

*"The best code is the code that makes you question your life choices"* - Anonymous Software Architect

**Disclaimer**: No bash scripts were harmed in the making of this application. All complexity was added voluntarily and with full knowledge of the consequences.

**Note**: I'm so hard-headed that I debated for three days whether to use AI to help write the README. In the end, I gave in and decided to let the AI write the entire thing (like the whole CLI). All hail [vibe coding](https://vibemanifesto.org/).


**Even more notes just because I like this note at the bottom thingy**: I'll probably add gifs (and emojis) on how to use it later.

**On a more serious note**: I do use this on a daily basis LOL. Just because my memory fails me and is convenient.