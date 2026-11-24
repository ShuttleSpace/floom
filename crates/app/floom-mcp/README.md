# floom-mcp

A [Model Context Protocol](https://modelcontextprotocol.io) server that exposes a
running Floom app to an agent: read the tree, read the semantics, tap a node by
its key, type into a field, read the frame that was presented.

It attaches to an app that already serves the agent protocol; it never starts one.

```
floom-window-software --agent 4877      # in one terminal
floom-mcp --attach 127.0.0.1:4877       # on the harness's stdio
```

Tools: `floom_tree`, `floom_semantics`, `floom_tap`, `floom_type`,
`floom_screenshot`. The first two are how an agent finds the key the third takes.

`FLOOM_AGENT_TOKEN` is passed through as the dev token when the app requires one.
