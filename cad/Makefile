all: one-key-platform.stl \
	 three-key-platform.stl \
	 twelve-key-platform.stl \
	 white-plunger-extension.stl \
	 black-plunger-extension.stl \
	 solenoid-support.stl \
	 winding-spindle.stl \
	 winding-support.stl \
	 steel-support.stl

one-key-platform.stl three-key-platform.stl twelve-key-platform.stl: key-platform.py common.py
	python3 key-platform.py
white-plunger-extension.stl black-plunger-extension.stl: plunger-extension.py common.py
	python3 plunger-extension.py
solenoid-support.stl: solenoid-support.py common.py
	python3 solenoid-support.py
winding-spindle.stl winding-support.stl: solenoid-winder.py common.py
	python3 solenoid-winder.py
steel-support.stl: steel-support.py common.py
	python3 steel-support.py

view:
	f3d --watch *.stl

clean:
	rm *.stl
