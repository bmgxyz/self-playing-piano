import cadquery as cq
from common import *

solenoid_support = (
    cq.Workplane("XY")
    .rect(
        SteelSupport.screw_hole_separation
        + M3Screw.hole_buffer_diameter,
        SolenoidSupport.inner_diameter
        + SolenoidSupport.wall_thickness * 2
    )
    .circle(SolenoidSupport.inner_diameter / 2)
    .pushPoints([
        ( SteelSupport.screw_hole_separation / 2, 0),
        (-SteelSupport.screw_hole_separation / 2, 0)
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
    .moveTo(SolenoidSupport.inner_diameter / 2 + SolenoidSupport.wall_thickness, 0)
    .hLine(-SolenoidSupport.wall_thickness)
    .vLine(Solenoid.height * SolenoidSupport.taper_fraction)
    .hLine(SolenoidSupport.wall_thickness + SolenoidSupport.taper_size)
    .close()
    .revolve()
)

export_stl(solenoid_support, "solenoid-support")
