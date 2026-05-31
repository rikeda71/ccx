---
name: deploy
description: Deploy the application to the target environment
when_to_use: Use this skill when the user wants to deploy the application
allowed-tools:
  - "Bash(git add *)"
  - "Bash(git commit -m)"
  - "Write(**/*.yaml)"
  - "Read(**/*.json)"
model: claude-opus-4-8
effort: max
user-invocable: true
paths:
  - "**/*.yaml"
  - "**/*.json"
---

# Deploy Skill

This skill deploys the application.

Use $ARGUMENTS[0] as the target environment (e.g. staging or prod).

Run this with /deploy staging

${CLAUDE_SESSION_ID} is the session identifier.

!`git status` to check current state.
