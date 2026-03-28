## [0.3.1] - 2026-03-28

### 🚀 Features

- Implement version 0.0.1
- Inital support i18n
- Version 0.0.2 (maybe?)
- Autocomplete for text input
- Adding bash command repeat to workflow
- Allowing user to specify a workflow url
- Copying command to clipboard
- Rollout pods workflow
- Going brrr unnecessarily
- Implement RocksDB storage backend and related commands
- Add commands for listing aggregates and replaying events, enhance storage backend with RocksDB support
- Add purge storage command and enhance RocksDB integration with shared instance management
- Add event types for aggregates listing and replaying, update language files to remove restart prompt
- Integrate tabled for improved state display in workflow commands and add new dependencies
- Enhance workflow state display with structured table output and localization support
- Add localization support for custom input prompts and command execution messages
- Implement ArgumentResolver for interactive workflow argument handling and dynamic command execution
- Add MultiEnum variant and selection constraint fields
- Add multi-select i18n translation keys
- Add MultiEnum resolution with multi-select support
- Add port trait abstractions (UserPrompt, CommandExecutor, FileSystem) with real and mock implementations
- Add Fumadocs documentation site and update README
- Warm dark theme, theme-aware nav icon, and netlify config
- Add smooth page transitions and micro-interactions to docs
- Restructure docs and add OpenGraph images
- Switch to cliclack for polished CLI prompts
- Improve storage UX — base62 IDs, table list, delete command
- Add changelog generation and GitHub release workflow

### 🐛 Bug Fixes

- Issues with dynamic values on enum
- Command runner is transparent now
- Correcting README.md, becuase AI made a mistake WHAT?
- Removing vibe coded messages
- Update state retrieval and event logging in ReplayAggregateCommand
- Update README to clarify event streaming terminology (btw, I like this ai generated commit msgs)
- *(deps)* Update rust crate whoami to v2 (#1)
- Update whoami v2 API calls for username and hostname
- Correct invalid state transitions in storage tests
- Correct broken doctests in i18n macros and loader
- Increase command timeout from 30s to 5 minutes
- Default sync URL to HTTPS instead of SSH
- *(ci)* Add libxcb dependencies for clipboard crate on Linux
- Correct netlify.toml paths for site/ subdirectory
- Add @netlify/plugin-nextjs as explicit dependency
- Add docs content and switch to static export for Netlify
- Remove copy/open buttons from docs, restore static search
- Center logo in OG image

### 🚜 Refactor

- Proper separation of concerns
- Better error messages for user
- Improve snapshot handling and event replay logic in RocksDbJournal
- Enhance command processing by loading command data and updating effect method signatures
- Remove AggregatesListedEvent and related functionality from command and event handling
- Update command processing to include loaded data and apply events to rebuild workflow state
- Extract execute_enum_command helper in resolver
- Split adapter commands into individual files with unit tests
- Extract ListWorkflowsCommand to list.rs with tests
- Extract StartWorkflowCommand to start.rs with tests
- Extract ResolveArgumentsCommand to resolve.rs with tests
- Extract CompleteWorkflowCommand to complete.rs with tests
- Extract remaining commands (sync, language, storage, aggregate) with tests
- Clean TUI output — remove emoji spam and redundant messages
- Remove banner comments and format code

### 📚 Documentation

- Add multi-select (MultiEnum) design spec

### 🎨 Styling

- Format code for consistency in event structs and remove unnecessary whitespace in purge storage command

### 🧪 Testing

- Add event application tests for all state transitions
- Add workflow and argument parsing tests
- Add i18n text manager and parameter substitution tests
- Add error handling and conversion tests
- Add event domain type and metadata tests

### ⚙️ Miscellaneous Tasks

- Removing comments
- Moving away from dialoguer in favour of inquiry
- Removing unused argument
- Adding more workflows
- Remove resource folder
- Adding readme to explain usage (LOL)
- Hate when I have to fix things cause AI didn't do them in the first try???
- Some needed clarifications
- Adding companion project
- Fix clippy warnings
- Update dependencies and improve async command execution
- Bump dependencies
- Add renovate to workflow
- Remove docs/ from git and add to gitignore
- Bump version to 0.2.0
- Bump package version
- Add GitHub Actions for wf::build and wf::test with badges
- Remove duplicate rustfmt.toml and clean up site README
- Bump version to 0.2.1
- Update Cargo.lock for 0.2.1
- Bump package version
- Bump version to 0.3.1
