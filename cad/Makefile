all: one-key-platform.stl \
	 three-key-platform.stl \
	 eleven-key-platform.stl \
	 white-plunger-extension.stl \
	 black-plunger-extension.stl \
	 solenoid-support.stl \
	 winding-spindle.stl \
	 winding-support.stl \
	 steel-support.stl

one-key-platform.stl three-key-platform.stl eleven-key-platform.stl: key-platform.py
	python3 key-platform.py
white-plunger-extension.stl black-plunger-extension.stl: plunger-extension.py
	python3 plunger-extension.py
solenoid-support.stl: solenoid-support.py
	python3 solenoid-support.py
winding-spindle.stl winding-support.stl: solenoid-winder.py
	python3 solenoid-winder.py
steel-support.stl: steel-support.py
	python3 steel-support.py

clean:
	rm *.stl