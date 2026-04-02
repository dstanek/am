# CI/CD Integration

This guide shows how to integrate `am` into continuous integration and deployment pipelines. Examples cover GitHub Actions, GitLab CI, and Jenkins.

---

## Overview

`am` is designed for interactive agent sessions, but can also be used in CI/CD pipelines to:

- Automate feature development with coding agents
- Run parallel agent sessions for complex tasks
- Generate code, tests, or documentation automatically
- Integrate agent output into your build and deployment workflows

All of the following examples use `am run` to execute commands inside an agent container without requiring tmux interaction.

---

## GitHub Actions

### Basic Setup

Create a workflow file `.github/workflows/agent-task.yml`:

```yaml
name: Agent Task

on: [push, pull_request]

jobs:
  agent-task:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0  # Full history needed for worktrees

      - name: Install am
        run: |
          curl -fsSL https://raw.githubusercontent.com/dstanek/am/main/install.sh | bash

      - name: Initialize am
        run: |
          am init

      - name: Set up Claude authentication
        env:
          ANTHROPIC_API_KEY: ${{ secrets.ANTHROPIC_API_KEY }}
        run: |
          mkdir -p ~/.claude

      - name: Run agent task
        env:
          ANTHROPIC_API_KEY: ${{ secrets.ANTHROPIC_API_KEY }}
        run: |
          am start agent-task --agent claude
          am run agent-task -- your-command-here
          am destroy agent-task --force

      - name: Commit and push results (optional)
        if: github.event_name == 'push'
        run: |
          git config user.name "Agent"
          git config user.email "agent@example.com"
          git add .
          git commit -m "Agent-generated changes" --allow-empty || true
          git push
```

### Parallel Agent Sessions

Run multiple agent sessions in parallel:

```yaml
jobs:
  parallel-agents:
    runs-on: ubuntu-latest
    strategy:
      matrix:
        agent: [claude, copilot]
        task: [tests, docs]
    steps:
      - uses: actions/checkout@v4

      - name: Install am
        run: curl -fsSL https://raw.githubusercontent.com/dstanek/am/main/install.sh | bash

      - name: Initialize am
        run: am init

      - name: Run parallel tasks
        env:
          ANTHROPIC_API_KEY: ${{ secrets.ANTHROPIC_API_KEY }}
          OPENAI_API_KEY: ${{ secrets.OPENAI_API_KEY }}
        run: |
          am start task-${{ matrix.agent }}-${{ matrix.task }} --agent ${{ matrix.agent }}
          am run task-${{ matrix.agent }}-${{ matrix.task }} -- your-command
          am destroy task-${{ matrix.agent }}-${{ matrix.task }} --force
```

### Pre-built Container Images

To speed up workflows, use pre-built agent images:

```yaml
steps:
  - name: Pull Claude image
    run: docker pull ghcr.io/dstanek/am-claude:latest

  - name: Run with pre-built image
    env:
      ANTHROPIC_API_KEY: ${{ secrets.ANTHROPIC_API_KEY }}
    run: |
      am start task --agent claude --image ghcr.io/dstanek/am-claude:latest
      am run task -- your-command
      am destroy task --force
```

---

## GitLab CI

### Basic Pipeline

Create `.gitlab-ci.yml`:

```yaml
stages:
  - agent-task

agent-task:
  stage: agent-task
  image: ubuntu:latest
  before_script:
    - apt-get update && apt-get install -y curl git
    - curl -fsSL https://raw.githubusercontent.com/dstanek/am/main/install.sh | bash
    - am init
    - mkdir -p ~/.claude
  script:
    - am start agent-task --agent claude
    - am run agent-task -- your-command-here
    - am destroy agent-task --force
  variables:
    ANTHROPIC_API_KEY: $ANTHROPIC_API_KEY
  artifacts:
    paths:
      - output/
    expire_in: 1 week
```

### Matrix Builds

Run multiple agent tasks in parallel:

```yaml
agent-matrix:
  stage: agent-task
  parallel:
    matrix:
      - AGENT: [claude, copilot]
        TASK: [tests, docs]
  script:
    - am init
    - am start task-$AGENT-$TASK --agent $AGENT
    - am run task-$AGENT-$TASK -- your-command
    - am destroy task-$AGENT-$TASK --force
  environment:
    name: $AGENT-$TASK
```

---

## Jenkins

### Declarative Pipeline

Create a `Jenkinsfile`:

```groovy
pipeline {
    agent any

    environment {
        ANTHROPIC_API_KEY = credentials('anthropic-api-key')
    }

    stages {
        stage('Setup') {
            steps {
                sh '''
                    curl -fsSL https://raw.githubusercontent.com/dstanek/am/main/install.sh | bash
                    am init
                    mkdir -p ~/.claude
                '''
            }
        }

        stage('Agent Task') {
            steps {
                sh '''
                    am start agent-task --agent claude
                    am run agent-task -- your-command-here
                    am destroy agent-task --force
                '''
            }
        }

        stage('Commit Results') {
            when {
                branch 'main'
            }
            steps {
                sh '''
                    git config user.name "Jenkins Agent"
                    git config user.email "jenkins@example.com"
                    git add .
                    git commit -m "Agent-generated changes [skip ci]" --allow-empty || true
                    git push origin HEAD:main
                '''
            }
        }
    }

    post {
        always {
            cleanWs()
        }
    }
}
```

### Scripted Pipeline

For more control, use a scripted approach:

```groovy
node {
    try {
        stage('Setup') {
            sh 'curl -fsSL https://raw.githubusercontent.com/dstanek/am/main/install.sh | bash'
            sh 'am init'
        }

        stage('Agent Task') {
            withEnv(['ANTHROPIC_API_KEY=' + credentials('anthropic-api-key')]) {
                sh 'am start build-agent --agent claude'
                sh 'am run build-agent -- your-command'
                sh 'am destroy build-agent --force'
            }
        }

        stage('Publish') {
            sh 'publish-results.sh'
        }
    } finally {
        cleanWs()
    }
}
```

---

## Best Practices

### 1. Use Secrets for API Keys

Store API keys in your CI/CD platform's secret management:

| Platform | Storage |
|----------|---------|
| GitHub Actions | Secrets (Settings > Secrets > Actions) |
| GitLab CI | Variables (Settings > CI/CD > Variables, mark as Protected/Masked) |
| Jenkins | Credentials (Manage Jenkins > Credentials) |

**Reference in pipelines:**

```yaml
# GitHub Actions
env:
  ANTHROPIC_API_KEY: ${{ secrets.ANTHROPIC_API_KEY }}

# GitLab CI
variables:
  ANTHROPIC_API_KEY: $ANTHROPIC_API_KEY

# Jenkins
withEnv(['ANTHROPIC_API_KEY=' + credentials('anthropic-api-key')])
```

### 2. Clean Up Sessions

Always destroy sessions after use, even if the task fails:

```yaml
# GitHub Actions
run: |
  am start task --agent claude
  am run task -- your-command || true  # Don't fail if command fails
  am destroy task --force

# GitLab CI
script:
  - am start task --agent claude
  - am run task -- your-command || true
  - am destroy task --force

# Jenkins
try {
  sh 'am start task --agent claude'
  sh 'am run task -- your-command'
} finally {
  sh 'am destroy task --force'
}
```

### 3. Use Unique Slugs

Prevent slug collisions when running parallel jobs:

```yaml
# GitHub Actions
run: |
  SLUG="job-${{ github.job }}-${{ github.run_number }}-${{ strategy.job-index }}"
  am start $SLUG --agent claude

# GitLab CI
script:
  - SLUG="job-$CI_JOB_NAME-$CI_PIPELINE_ID"
  - am start $SLUG --agent claude

# Jenkins
def slug = "job-${env.JOB_NAME}-${env.BUILD_ID}"
sh "am start ${slug} --agent claude"
```

### 4. Cache Container Images

Speed up pipelines by pre-pulling or caching images:

```yaml
# GitHub Actions
- name: Cache Claude image
  uses: docker/setup-buildx-action@v2

- name: Pull image
  run: docker pull ghcr.io/dstanek/am-claude:latest

# GitLab CI
cache:
  paths:
    - /root/.docker/
  policy: pull-push

before_script:
  - docker pull ghcr.io/dstanek/am-claude:latest

# Jenkins (Docker plugin)
agent {
    docker {
        image 'ghcr.io/dstanek/am-claude:latest'
    }
}
```

### 5. Capture Output

Save agent output for logs and artifacts:

```sh
# Redirect output to a file
am run task -- your-command > output.log 2>&1

# Capture in a variable (GitHub Actions)
- name: Run task
  id: agent-task
  run: |
    OUTPUT=$(am run task -- your-command)
    echo "output=$OUTPUT" >> $GITHUB_OUTPUT
    echo $OUTPUT > task-output.txt

# Use artifacts
artifacts:
  paths:
    - task-output.txt
  reports:
    dotenv: task-output.txt
```

### 6. Handle Agent Failures Gracefully

Decide whether agent failures should fail the pipeline:

```sh
# Option 1: Fail on agent error (default)
am run task -- your-command

# Option 2: Continue even if agent fails
am run task -- your-command || true

# Option 3: Fail only for specific exit codes
set -e
am run task -- your-command || EXIT_CODE=$?
if [ $EXIT_CODE -ne 0 ] && [ $EXIT_CODE -ne 42 ]; then
    exit $EXIT_CODE
fi
```

---

## Example Workflows

### Generate Documentation with Claude

```yaml
# .github/workflows/generate-docs.yml
name: Generate Docs

on:
  push:
    branches: [main]

jobs:
  generate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: curl -fsSL https://raw.githubusercontent.com/dstanek/am/main/install.sh | bash
      - run: am init
      - env:
          ANTHROPIC_API_KEY: ${{ secrets.ANTHROPIC_API_KEY }}
        run: |
          am start docs --agent claude
          am run docs -- claude "Generate API documentation for src/"
          am destroy docs --force
      - uses: actions/upload-artifact@v4
        with:
          name: generated-docs
          path: docs/
```

### Write Tests with Copilot

```yaml
# .github/workflows/generate-tests.yml
name: Generate Tests

on:
  pull_request:

jobs:
  tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: curl -fsSL https://raw.githubusercontent.com/dstanek/am/main/install.sh | bash
      - run: am init
      - env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        run: |
          am start tests --agent copilot
          am run tests -- copilot "Write unit tests for src/main.rs"
          am destroy tests --force
      - run: cargo test
```

---

## Troubleshooting CI/CD Integration

### Session creation fails

Ensure git is initialized and tmux is not required for CI/CD:

```sh
# Skip tmux if not available
am start task --agent claude --no-tmux  # (if this flag exists)

# Or use a custom command that doesn't need tmux
am run task -- your-command
```

### API key not accessible in container

Verify that secrets are passed as environment variables:

```yaml
# Correct: Environment variable
env:
  ANTHROPIC_API_KEY: ${{ secrets.ANTHROPIC_API_KEY }}
run: |
  echo $ANTHROPIC_API_KEY  # Should print your key
  am start task --agent claude

# Wrong: Hardcoded in script
run: |
  am start task --agent claude --api-key sk-...  # Don't do this!
```

### Container cleanup fails

Always use `--force` to skip prompts in non-interactive environments:

```sh
am destroy task --force
```

### Docker/Podman not available in CI

Use the system package manager:

```yaml
before_script:
  - apt-get update && apt-get install -y docker.io podman
  - systemctl start docker  # If needed
```

---

## Next Steps

- Learn more about [agent-specific configuration](../reference/configuration.md)
- Customize images for your CI/CD environment: [Custom Container Images](custom-images.md)
- Check out [Commands Reference](../reference/commands.md) for all available options
