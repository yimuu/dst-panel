# docker-compose.yml 文件参考

```yaml
version: "3.8"

services:
  dst-admin-rust:
    image: yimuu/dst-panel:latest
    container_name: dst-admin-rust
    restart: unless-stopped
    ports:
      - "8082:8082"
      - "10888:10888/udp"
      - "10998:10998/udp"
      - "10999:10999/udp"
    volumes:
      - ${PWD}/dstsave:/data
    environment:
      - TZ=Asia/Shanghai
```
