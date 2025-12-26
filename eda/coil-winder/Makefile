all: gerber pcb2gcode process_gcode

gerber: coil-winder.kicad_pcb
	mkdir -p gerber
	cd gerber && \
	kicad-cli pcb export gerbers -l F.Cu,Edge.Cuts ../coil-winder.kicad_pcb && \
	kicad-cli pcb export drill --format excellon ../coil-winder.kicad_pcb && \
	cd ..

pcb2gcode: gerber/coil-winder.drl \
		   gerber/coil-winder-Edge_Cuts.gm1 \
		   gerber/coil-winder-F_Cu.gtl \
		   gerber/coil-winder-job.gbrjob
	mkdir -p gcode
	pcb2gcode \
		--drill gerber/coil-winder.drl \
		--front gerber/coil-winder-F_Cu.gtl \
		--outline gerber/coil-winder-Edge_Cuts.gm1 \
		--basename coil-winder
	rm gcode/*.svg

process_gcode: gcode/tracks.gcode gcode/drill.gcode gcode/outline.gcode

dx := -1.22496
dy := -1.22501
xm := 25.20001

gcode/tracks.gcode: gerber/coil-winder-F_Cu.gtl gerber/coil-winder-job.gbrjob
	gctk translate -x $(dx) -y $(dy) < ./gcode/coil-winder_front.ngc \
	| gctk mirror -x $(xm) > ./gcode/tracks.gcode
	rm ./gcode/coil-winder_front.ngc

gcode/drill.gcode: gerber/coil-winder.drl gerber/coil-winder-job.gbrjob
	gctk translate -x $(dx) -y $(dy) < ./gcode/coil-winder_milldrill.ngc \
	| gctk mirror -x $(xm) > ./gcode/drill.gcode
	rm ./gcode/coil-winder_milldrill.ngc

gcode/outline.gcode: gerber/coil-winder-Edge_Cuts.gm1 gerber/coil-winder-job.gbrjob
	gctk translate -x $(dx) -y $(dy) < ./gcode/coil-winder_outline.ngc \
	| gctk mirror -x $(xm) > ./gcode/outline.gcode
	rm ./gcode/coil-winder_outline.ngc

clean:
	rm -rf gerber
	rm -rf gcode
