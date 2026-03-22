// Workspace list actions (rendering is inlined in app.rs)

pub enum WorkspaceAction {
    Switch(usize),
    Create,
    Delete(usize),
}
