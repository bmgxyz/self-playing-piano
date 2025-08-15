all: gerber pcb2gcode process_gcode

gerber: midi-driver.kicad_pcb
	mkdir -p gerber
	cd gerber && \
	kicad-cli pcb export gerbers -l F.Cu,Edge.Cuts ../midi-driver.kicad_pcb && \
	kicad-cli pcb export drill --format excellon ../midi-driver.kicad_pcb && \
	cd ..

pcb2gcode: gerber/midi-driver.drl \
		   gerber/midi-driver-Edge_Cuts.gm1 \
		   gerber/midi-driver-F_Cu.gtl \
		   gerber/midi-driver-job.gbrjob
	mkdir -p gcode
	pcb2gcode \
		--drill gerber/midi-driver.drl \
		--front gerber/midi-driver-F_Cu.gtl \
		--outline gerber/midi-driver-Edge_Cuts.gm1 \
		--basename midi-driver
	rm gcode/*.svg

process_gcode: gcode/tracks.gcode gcode/drill.gcode gcode/outline.gcode

dx := -1.22496
dy := -1.225
xm := 38.700015

gcode/tracks.gcode: gerber/midi-driver-F_Cu.gtl gerber/midi-driver-job.gbrjob
	gctk translate -x $(dx) -y $(dy) < ./gcode/midi-driver_front.ngc \
	| gctk mirror -x $(xm) > ./gcode/tracks.gcode
	rm ./gcode/midi-driver_front.ngc

gcode/drill.gcode: gerber/midi-driver.drl gerber/midi-driver-job.gbrjob
	gctk translate -x $(dx) -y $(dy) < ./gcode/midi-driver_milldrill.ngc \
	| gctk mirror -x $(xm) > ./gcode/drill.gcode
	rm ./gcode/midi-driver_milldrill.ngc

gcode/outline.gcode: gerber/midi-driver-Edge_Cuts.gm1 gerber/midi-driver-job.gbrjob
	gctk translate -x $(dx) -y $(dy) < ./gcode/midi-driver_outline.ngc \
	| gctk mirror -x $(xm) > ./gcode/outline.gcode
	rm ./gcode/midi-driver_outline.ngc

clean:
	rm -rf gerber
	rm -rf gcode
