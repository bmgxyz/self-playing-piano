all: key-support-octave.stl \
	 key-support-low.stl \
	 key-support-high.stl \
	 white-plunger-extension.stl \
	 black-plunger-extension.stl \
	 solenoid-support.stl

key-support-octave.stl \
key-support-low.stl \
key-support-high.stl &: key-support.py common.py
	uv run python key-support.py -e
end-support.stl \
end-support-spacer.stl &: end-support.py common.py
	uv run python end-support.py -e
white-plunger-extension.stl black-plunger-extension.stl: plunger-extension.py common.py
	uv run python plunger-extension.py
solenoid-support.stl: solenoid-support.py common.py
	uv run python solenoid-support.py

view:
	f3d --watch *.stl

clean:
	rm -rf ./*.stl
