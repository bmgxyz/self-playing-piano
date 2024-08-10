import cadquery as cq
from common import *

solenoid_support = (
    cq.Workplane("XY")
    .rect(
        SolenoidSupport.screw_hole_separation
        + M3Screw.hole_buffer_diameter,
        SolenoidSupport.inner_diameter
        + SolenoidSupport.wall_thickness * 2
    )
    .circle(SolenoidSupport.inner_diameter / 2)
    .pushPoints([
        ( SolenoidSupport.screw_hole_separation / 2, 0),
        (-SolenoidSupport.screw_hole_separation / 2, 0)
    ])
    .circle(M3Screw.hole_diameter / 2)
    .extrude(SolenoidSupport.base_thickness)
    .faces(">Z")
    .workplane()
    .circle(SolenoidSupport.inner_diameter / 2)
    .circle(
        SolenoidSupport.inner_diameter / 2
        + SolenoidSupport.wall_thickness
    )
    .extrude(Solenoid.height * (1 - SolenoidSupport.taper_fraction))
    .faces(">Z")
    .workplane()
    .transformed(rotate=(90, 0, 0))
    .moveTo(SolenoidSupport.inner_diameter / 2, 0)
    .line(
        SolenoidSupport.taper_size,
        Solenoid.height * SolenoidSupport.taper_fraction
    )
    .line(SolenoidSupport.wall_thickness, 0)
    .line(
        -SolenoidSupport.taper_size,
        -Solenoid.height * SolenoidSupport.taper_fraction
    )
    .close()
    .revolve()
)

export_stl(solenoid_support, "solenoid-support")
