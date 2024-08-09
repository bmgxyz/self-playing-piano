mfemm_setup();
close all;

## All physical quantities are represented in base SI units (kg, m, s, A, etc.).

## Independent Variables ##

CoilBlockProps.Turns = 260;
       coil.pack_eff = 0.65;
    coil.length_corr = 0.76;
            coil.gap = 1.35e-3;
         coil.height = 32e-3;
      coil.wire_diam = 0.644e-3;
        coil.voltage = 24;
coil.wire_diam_w_ins = 0.660e-3;

      plunger.radius = 4.7625e-3;
      plunger.height = 42e-3;
           plunger.y = -coil.height/2 - plunger.height/2;

     frame.thickness = 2e-3;
           frame.gap = 1e-3;

## Constants ##

solution_radius = 120e-3;
mu_0 = 4e-7 * pi;
steel_density = 7.861e3;

## Problem Setup ##

AirBlockProps.BlockType = 'Air';

CoilBlockProps.BlockType = 'Copper';
CoilBlockProps.InCircuit = 'Coil';
coil.resistivity = 1.68e-8; # copper
coil.wire_area = (coil.wire_diam/2).^2 * pi;
coil.wire_area_w_ins = (coil.wire_diam_w_ins/2).^2 * pi;
coil.x = plunger.radius + coil.gap;
coil.y = -coil.height/2;
coil.width = CoilBlockProps.Turns * coil.wire_area_w_ins / (coil.height * coil.pack_eff);
coil.radius = coil.x + coil.width;
coil.approx_circumference = coil.radius * 2 * pi;
coil.approx_length = coil.approx_circumference * CoilBlockProps.Turns * coil.length_corr;
coil.approx_resistance = coil.approx_length / coil.wire_area * coil.resistivity;
coil.current = coil.voltage / coil.approx_resistance;
coil.approx_inductance = 250 * mu_0 * CoilBlockProps.Turns * pi * coil.radius .^ 2 / coil.height;
coil.area = (coil.radius - coil.x) * coil.height;
coil.current_density = coil.current * CoilBlockProps.Turns * coil.area;

plunger.x = 0;
plunger.volume = plunger.height * plunger.radius.^2 * pi;
plunger.mass = plunger.volume * steel_density;
PlungerBlockProps.BlockType = '1018 Steel';
PlungerBlockProps.InGroup = 1;

frame.top_x = coil.x;
frame.top_y = coil.y + coil.height + frame.gap;
frame.top_width = coil.width + frame.thickness + frame.gap;
frame.top_height = frame.thickness;
frame.side_x = coil.x + coil.width + frame.gap;
frame.side_y = coil.y - frame.gap + 1e-4;
frame.side_width = frame.thickness;
frame.side_height = coil.height + frame.gap * 2 - 2e-4;
# These volume estimates assume rectangular sections, while the FEMM simulation
# assumes cylindrical; also, two of the side faces are uncovered
frame.volume = ((frame.top_x + frame.top_width)^2 - frame.top_x^2)*frame.thickness;
frame.volume += frame.thickness*coil.height*coil.radius*2*2;
frame.mass = steel_density * frame.volume;
FrameBlockProps.BlockType = PlungerBlockProps.BlockType;
FrameBlockProps.InGroup = PlungerBlockProps.InGroup;

FemmProblem = newproblem_mfemm('axi', ...
                               'Frequency', 0, ...
                               'LengthUnits', 'meters');

# Set solution boundary
boundary_nodes = [0, -solution_radius;
                  0,  solution_radius];
[FemmProblem, nodeinds, nodeids] = addnodes_mfemm(FemmProblem, boundary_nodes(:,1), boundary_nodes(:,2));
[FemmProblem] = addsegments_mfemm(FemmProblem, nodeids(1), nodeids(2));
[FemmProblem, rcsegind] = addarcsegments_mfemm(FemmProblem, nodeids(1), nodeids(2), 180, 'MaxSegDegrees', 2.5);

# Add coil
FemmProblem = addmaterials_mfemm(FemmProblem, CoilBlockProps.BlockType);
FemmProblem = addcircuit_mfemm(FemmProblem, 'Coil', 'TotalAmps_re', coil.current);
FemmProblem = addrectregion_mfemm(FemmProblem, coil.x, coil.y, coil.width, coil.height, CoilBlockProps);

# Add frame
FemmProblem = addmaterials_mfemm(FemmProblem, FrameBlockProps.BlockType);
FemmProblem = addrectregion_mfemm(FemmProblem, frame.top_x, frame.top_y, frame.top_width, frame.top_height, FrameBlockProps);
FemmProblem = addrectregion_mfemm(FemmProblem, frame.side_x, frame.side_y, frame.side_width, frame.side_height, FrameBlockProps);

# Add plunger
FemmProblem = addmaterials_mfemm(FemmProblem, PlungerBlockProps.BlockType);
[FemmProblem, seginds, nodeinds, blockind, nodeids, labelloc] = addrectregion_mfemm(FemmProblem, plunger.x, plunger.y, plunger.radius, plunger.height, PlungerBlockProps);

# Add air
FemmProblem = addblocklabel_mfemm(FemmProblem, 0.001, solution_radius-0.001, ...
                                  'BlockType', 'Air');

# Add boundary
[FemmProblem, boundind, boundname] = addboundaryprop_mfemm(FemmProblem, 'ABC', 2, ...
                                    'c0', 1/(mu_0*solution_radius), ...
                                    'c1', 0);

## Solve Problem ##

##plotfemmproblem(FemmProblem);
filename = 'solenoid_actuator.fem';
writefemmfile(filename, FemmProblem);
filename = fmesher(filename);
ansfile = fsolver(filename);
myfpproc = fpproc();
myfpproc.opendocument(ansfile);

## Compute Force ##

myfpproc.groupselectblock(1);
plunger_force = myfpproc.blockintegral(19);

## Estimate Cost ##

# per-volume costs are approximate; they generally decrease with increasing volume
cost.wire_per_vol = 3.29e5;
cost.plunger_per_vol = 9.2e4;
cost.frame_per_vol = 3.5e5;
cost.plunger = plunger.volume * cost.plunger_per_vol;
cost.wire = cost.wire_per_vol * coil.approx_length * coil.wire_area;
cost.frame = frame.volume * cost.frame_per_vol;
cost.total = cost.plunger + cost.wire + cost.frame;

## Print Results ##

coil
CoilBlockProps
plunger
PlungerBlockProps
frame
FrameBlockProps
cost
plunger_force
