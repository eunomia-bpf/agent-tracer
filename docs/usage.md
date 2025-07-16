# usage

## claude code

```sh
sudo /home/yunwei37/agent-tracer/collector/target/release/collector ssl --http-parser --http-filter "request.path_prefix=/v1/rgstr | response.status_code=202 | request.method=HEAD | response.body=" --ssl-filter "data=0\r\n\r\n"
```

```sh
sudo /home/yunwei37/agent-tracer/collector/target/release/collector agent -c "claude" --http-parser --http-filter "request.path_prefix=/v1/rgstr | response.status_code=202 | request.method=HEAD | response.body=" --ssl-filter "data=0\r\n\r\n"
```
