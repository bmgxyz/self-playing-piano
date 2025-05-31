all: key-platform-0.stl \
	 key-platform-1.stl \
	 key-platform-2.stl \
	 key-platform-3.stl \
	 key-platform-4.stl \
	 key-platform-5.stl \
	 key-platform-6.stl \
	 key-platform-7.stl \
	 white-plunger-extension.stl \
	 black-plunger-extension.stl \
	 solenoid-support.stl

key-platform-0.stl \
key-platform-1.stl \
key-platform-2.stl \
key-platform-3.stl \
key-platform-4.stl \
key-platform-5.stl \
key-platform-6.stl \
key-platform-7.stl: key-platform.py common.py
	python3 key-platform.py -e
white-plunger-extension.stl black-plunger-extension.stl: plunger-extension.py common.py
	python3 plunger-extension.py
solenoid-support.stl: solenoid-support.py common.py
	python3 solenoid-support.py

view:
	f3d --watch *.stl

clean:
	rm -rf ./*.stl
