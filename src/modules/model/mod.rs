// Legacy solid-command implementations are kept temporarily for DWG import
// compatibility, but OCS2Cam does not register a solid-modelling ribbon or
// expose these commands.
pub mod boolean_cmd;
pub mod cylinder_cmd;
pub mod edge_cmd;
pub mod polysolid_cmd;
pub mod primitive_cmd;
pub mod sectionplane_cmd;
pub mod shell_cmd;
pub mod slice_cmd;
