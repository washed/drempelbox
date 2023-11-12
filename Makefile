.PHONY: build build-aarch64-unknown-linux-gnu build-x86_64-unknown-linux-gnu copy bca service file track album playlist artist stop

build: build-aarch64-unknown-linux-gnu build-x86_64-unknown-linux-gnu

build-aarch64-unknown-linux-gnu:
	cross build --release --target=aarch64-unknown-linux-gnu

build-x86_64-unknown-linux-gnu:
	cross build --release --target=x86_64-unknown-linux-gnu

copy:
	scp service/drempelbox.service ${RPI_HOST}:${RPI_TEMP_PATH}/drempelbox.service
	scp target/aarch64-unknown-linux-gnu/release/drempelbox ${RPI_HOST}:${RPI_TEMP_PATH}/drempelbox

	ssh ${RPI_HOST} sudo systemctl stop drempelbox

	ssh ${RPI_HOST} sudo mv ${RPI_TEMP_PATH}/drempelbox /usr/bin/drempelbox
	ssh ${RPI_HOST} sudo mv ${RPI_TEMP_PATH}/drempelbox.service /etc/systemd/system/drempelbox.service

	ssh ${RPI_HOST} sudo systemctl daemon-reload
	ssh ${RPI_HOST} sudo systemctl restart drempelbox

bca: build-aarch64-unknown-linux-gnu copy

service:
	ssh ${RPI_HOST} sudo systemctl enable drempelbox

file:
	curl -X POST -G "http://${CURL_TEST_HOST_PORT}/url" --data-urlencode 'url=file://./audio/police_s.wav'

file2:
	curl -X POST -G "http://${CURL_TEST_HOST_PORT}/url" --data-urlencode 'url=file://./audio/Duel of the Fates.mp3'

track:
	curl -X POST -G "http://${CURL_TEST_HOST_PORT}/url" --data-urlencode 'url=https://open.spotify.com/track/4abJbqX8C8CQTXHZxEbJZz?si=f04b62b8e85c4bf1'

album:
	curl -X POST -G "http://${CURL_TEST_HOST_PORT}/url" --data-urlencode 'url=https://open.spotify.com/album/4Gfnly5CzMJQqkUFfoHaP3\?si\=Qc1c-X7lTlG8-W8pdZLE3g'

playlist:
	curl -X POST -G "http://${CURL_TEST_HOST_PORT}/url" --data-urlencode 'url=https://open.spotify.com/playlist/62Q9JugytREDtl4i4fcHfX?si=85b818e3b652440f'

artist:
	curl -X POST -G "http://${CURL_TEST_HOST_PORT}/url" --data-urlencode 'url=https://open.spotify.com/artist/2RSApl0SXcVT8Yiy4UaPSt?si=deqOijWTSRa49exTMfUPDQ'

stop:
	curl -X POST -G "http://${CURL_TEST_HOST_PORT}/stop"

volume_up:
	curl -X POST -G "http://${CURL_TEST_HOST_PORT}/volume/up"

volume_down:
	curl -X POST -G "http://${CURL_TEST_HOST_PORT}/volume/down"

volume_set:
	curl -X POST -G "http://${CURL_TEST_HOST_PORT}/volume/set" --data-urlencode 'volume=$(volume)'

amp_on:
	curl -X POST -G "http://${CURL_TEST_HOST_PORT}/amp/on"

amp_off:
	curl -X POST -G "http://${CURL_TEST_HOST_PORT}/amp/off"
