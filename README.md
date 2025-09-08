# 🎭 Workflow CLI: The Most Overengineered YAML Runner in Existence

Welcome to the **Workflow CLI** - a terminal-native alternative to [Warp's Workflows](https://docs.warp.dev/knowledge-and-collaboration/warp-drive/workflows) that takes the simple concept of "parameterized commands from YAML files" and turns it into a distributed, actor-based, event-sourced, internationalized, fault-tolerant masterpiece of unnecessary complexity! 🚀

## What Does This Thing Actually Do?

At its core, this application does something absolutely revolutionary: it reads YAML workflow files (dynamicallyresolvescommandargumentsthroughinteractivepromptsbutwhocares) and... **copies the final command to your clipboard**. That's it. No execution. No running. Just good old-fashioned parameterized Ctrl+V material. Think of it as Warp Workflows for people who refuse to leave their beloved terminal.

## Features That Nobody Asked For

- 🎪 **Interactive Workflow Selection**: Choose your YAML workflow through a beautiful CLI menu (because `ls *.yaml` is for peasants)
- 🔧 **Dynamic Argument Resolution**: Interactive prompts for command parameters (just like Warp, but with 10x more code)
- 🎭 **Actor Supervision Trees**: Guardian actors watching WorkflowManager actors watching CommandProcessor actors (it's turtles all the way down)
- 📚 **Event Journaling**: Every parameter substitution is persisted as an event (because what if you need to replay that `kubectl get pods -n {{namespace}}` resolution?)
- 🌍 **Multi-language Support**: Parameter prompt errors in English AND Spanish (porque los errores de parámetros son internacionales)
- 🔄 **Command Chaining**: Workflows can trigger other workflows (because parameterized recursion is fun)
- 📦 **Pluggable Storage**: Swap between in-memory and... well, just in-memory for now (but the abstraction is there!)

## Usage

```bash
# The Warp way (modern but requires Warp)
# Use Warp's built-in workflow system with nice UI

# The simple way (boring)
# Manually edit your YAML files and copy-paste commands

# The ENTERPRISE way (exciting!)
workflow
# Interactive workflow selection → parameter prompts → clipboard magic!
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

Each YAML workflow selection spawns its own CommandProcessor actor with its own Engine instance and Journal for maximum isolation. Because you never know when resolving `kubectl get pods -n {{namespace}}` parameters might crash the entire system.

## Still Not Overengineered Enough?

Don't worry! There are still plenty of opportunities to add more unnecessary complexity:

- [ ] **Distributed Mode**: Why run on one machine when you can have a cluster?
- [ ] **Blockchain Integration**: Probably?
- [ ] **Machine Learning**: AI-powered YAML file recommendations, because the hype train might end soon
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

Companion project: https://github.com/sagoez/workflow-vault
