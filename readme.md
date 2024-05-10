# 🐻 Bear Proxy
Experimental Attack/Defence CTF proxy.

**Steps:** 
Create a docker network to connect all the containers with the proxy:
```bash
docker network create bear-proxy_internal
```

Add all the containers in the same network appending
```yml
networks:
  internal:
    external:
      name: bear-proxy_internal
```
in their `docker-compose.yml`. Also, don't forget to modify their exposed port.

In the `config.yml` use the container name as hostname of the services to proxy.
In the `docker-compose.yml` expose all the services ports.

Lastly, build and run the container:
```bash
docker compose up --build -d
```