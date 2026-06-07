# Development Documentation

## Testing

Testing is done using [`task`](https://taskfile.dev/) and the tasks in [`taskfile.yml`](./taskfile.yml).

Before a new version is released or a PR is accepted following tasks should run successfully:

```sh
task --parallel test:check test:integration test:clippy test:fmt doc
```

Logs of integration tests are written into `target/license-fetcher.log` and should be written on error onto console.

## Zed Setup

I recommend following extensions:

- Comments Highlighter
- Typos spell checker
- ltex
- Cyberpunk Scarlet
- log
