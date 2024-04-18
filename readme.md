**Steps:** 

Add all the containers in the same network appending
```yml
networks:
  internal:
    external:
      name: ctf-proxy_internal
```
in their `docker-compose.yml`. Also, don't forget to modify their exposed port.

In the `config.yml` use the container name as hostname of the services to proxy.
In the `docker-compose.yml` expose all the services ports.