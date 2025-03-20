host-run:
	sudo cargo r -- 10.0.0.2 192.168.197.2:8000 8001

container-run:
	cargo r -- 10.0.0.3 192.168.197.1:8001 8000

build-container-image:
	sudo docker build -t snoopy_docker_image .

remove-container-image:
	-docker stop server
	-docker rm server
	-docker image rm snoopy_docker_image

server-container-init:
	-docker stop server
	-docker rm server
	docker run --name server -it -v ~/Documents/code/playground/snoopy:/usr/src/snoopy --network=snoopy --cap-add=NET_ADMIN --privileged snoopy_docker_image

server-container-run:
	-docker start server
	-docker attach server

client-container:
	docker run --name client --rm -it --network=snoopy snoopy_docker_image
