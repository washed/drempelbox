.PHONY: build build-aarch64-unknown-linux-gnu build-x86_64-unknown-linux-gnu copy file track album playlist artist stop

build: build-aarch64-unknown-linux-gnu build-x86_64-unknown-linux-gnu

build-aarch64-unknown-linux-gnu:
	cross build --release --target=aarch64-unknown-linux-gnu

build-x86_64-unknown-linux-gnu:
	cross build --release --target=x86_64-unknown-linux-gnu

copy:
	scp target/aarch64-unknown-linux-gnu/release/drempelbox ${RPI_HOST}:${RPI_APP_PATH}/drempelbox

file:
	curl -X POST -G "http://${CURL_TEST_HOST_PORT}/url" --data-urlencode 'url=file://./audio/police_s.wav'

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
