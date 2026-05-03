# Development Documentation

## Testing

Testing is done using [`task`](https://taskfile.dev/) and the tasks in [`taskfile.yml`](./taskfile.yml).

Before a new version is released or a PR is accepted following tasks should run successfully:

```sh
task --parallel test:check test:integration doc
```

