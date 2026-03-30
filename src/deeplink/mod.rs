// Deep-link URL parsing and navigation for void:// protocol
//
// URL format:
//   void://open/<workspace_id>                    → switch to workspace
//   void://open/<workspace_id>/<panel_id>         → focus panel + center viewport
//   void://open/<workspace_id>/@<x>,<y>[,<zoom>]  → navigate to canvas coordinates

pub mod ipc;
#[cfg(target_os = "macos")]
pub mod macos;
pub mod register;
pub mod toast;

use std::fmt;

/// Parsed deep-link target.
#[derive(Debug, Clone, PartialEq)]
pub enum DeepLink {
    Workspace {
        workspace_id: String,
    },
    Panel {
        workspace_id: String,
        panel_id: String,
    },
    Position {
        workspace_id: String,
        x: f32,
        y: f32,
        zoom: Option<f32>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum DeepLinkError {
    InvalidScheme,
    MissingAction,
    MissingWorkspaceId,
    InvalidWorkspaceId,
    InvalidPanelId,
    InvalidCoordinates,
}

impl fmt::Display for DeepLinkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidScheme => write!(f, "URL must start with void://"),
            Self::MissingAction => write!(f, "missing action (expected void://open/...)"),
            Self::MissingWorkspaceId => write!(f, "missing workspace ID"),
            Self::InvalidWorkspaceId => write!(f, "invalid workspace UUID"),
            Self::InvalidPanelId => write!(f, "invalid panel UUID"),
            Self::InvalidCoordinates => {
                write!(f, "invalid coordinates (expected @x,y or @x,y,zoom)")
            }
        }
    }
}

/// Parse a `void://` deep-link URL into a navigation target.
pub fn parse(url: &str) -> Result<DeepLink, DeepLinkError> {
    // Strip scheme
    let rest = url
        .strip_prefix("void://")
        .ok_or(DeepLinkError::InvalidScheme)?;

    // Strip action
    let rest = rest
        .strip_prefix("open/")
        .ok_or(DeepLinkError::MissingAction)?;

    // Split into segments (filter empty for trailing slashes)
    let segments: Vec<&str> = rest.split('/').filter(|s| !s.is_empty()).collect();

    if segments.is_empty() {
        return Err(DeepLinkError::MissingWorkspaceId);
    }

    let workspace_id = segments[0];
    // Validate workspace UUID
    uuid::Uuid::parse_str(workspace_id).map_err(|_| DeepLinkError::InvalidWorkspaceId)?;
    let workspace_id = workspace_id.to_string();

    if segments.len() == 1 {
        return Ok(DeepLink::Workspace { workspace_id });
    }

    let second = segments[1];

    // Check if it's a coordinate segment (@x,y or @x,y,zoom)
    if let Some(coords) = second.strip_prefix('@') {
        let parts: Vec<&str> = coords.split(',').collect();
        if parts.len() < 2 || parts.len() > 3 {
            return Err(DeepLinkError::InvalidCoordinates);
        }
        let x: f32 = parts[0]
            .parse()
            .map_err(|_| DeepLinkError::InvalidCoordinates)?;
        let y: f32 = parts[1]
            .parse()
            .map_err(|_| DeepLinkError::InvalidCoordinates)?;
        let zoom: Option<f32> = if parts.len() == 3 {
            Some(
                parts[2]
                    .parse()
                    .map_err(|_| DeepLinkError::InvalidCoordinates)?,
            )
        } else {
            None
        };
        return Ok(DeepLink::Position {
            workspace_id,
            x,
            y,
            zoom,
        });
    }

    // Otherwise it's a panel ID
    uuid::Uuid::parse_str(second).map_err(|_| DeepLinkError::InvalidPanelId)?;
    Ok(DeepLink::Panel {
        workspace_id,
        panel_id: second.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const WS_ID: &str = "550e8400-e29b-41d4-a716-446655440000";
    const PANEL_ID: &str = "6ba7b810-9dad-11d1-80b4-00c04fd430c8";

    #[test]
    fn parse_workspace_url() {
        let url = format!("void://open/{WS_ID}");
        assert_eq!(
            parse(&url).unwrap(),
            DeepLink::Workspace {
                workspace_id: WS_ID.to_string()
            }
        );
    }

    #[test]
    fn parse_workspace_with_trailing_slash() {
        let url = format!("void://open/{WS_ID}/");
        assert_eq!(
            parse(&url).unwrap(),
            DeepLink::Workspace {
                workspace_id: WS_ID.to_string()
            }
        );
    }

    #[test]
    fn parse_panel_url() {
        let url = format!("void://open/{WS_ID}/{PANEL_ID}");
        assert_eq!(
            parse(&url).unwrap(),
            DeepLink::Panel {
                workspace_id: WS_ID.to_string(),
                panel_id: PANEL_ID.to_string(),
            }
        );
    }

    #[test]
    fn parse_position_without_zoom() {
        let url = format!("void://open/{WS_ID}/@500.5,300");
        assert_eq!(
            parse(&url).unwrap(),
            DeepLink::Position {
                workspace_id: WS_ID.to_string(),
                x: 500.5,
                y: 300.0,
                zoom: None,
            }
        );
    }

    #[test]
    fn parse_position_with_zoom() {
        let url = format!("void://open/{WS_ID}/@100,200,1.5");
        assert_eq!(
            parse(&url).unwrap(),
            DeepLink::Position {
                workspace_id: WS_ID.to_string(),
                x: 100.0,
                y: 200.0,
                zoom: Some(1.5),
            }
        );
    }

    #[test]
    fn reject_invalid_scheme() {
        assert_eq!(
            parse("http://open/abc").unwrap_err(),
            DeepLinkError::InvalidScheme
        );
    }

    #[test]
    fn reject_missing_action() {
        assert_eq!(
            parse("void://foo/bar").unwrap_err(),
            DeepLinkError::MissingAction
        );
    }

    #[test]
    fn reject_missing_workspace() {
        assert_eq!(
            parse("void://open/").unwrap_err(),
            DeepLinkError::MissingWorkspaceId
        );
    }

    #[test]
    fn reject_invalid_workspace_uuid() {
        assert_eq!(
            parse("void://open/not-a-uuid").unwrap_err(),
            DeepLinkError::InvalidWorkspaceId
        );
    }

    #[test]
    fn reject_invalid_panel_uuid() {
        let url = format!("void://open/{WS_ID}/not-a-uuid");
        assert_eq!(parse(&url).unwrap_err(), DeepLinkError::InvalidPanelId);
    }

    #[test]
    fn reject_invalid_coordinates() {
        let url = format!("void://open/{WS_ID}/@abc,def");
        assert_eq!(parse(&url).unwrap_err(), DeepLinkError::InvalidCoordinates);
    }

    #[test]
    fn reject_coordinates_too_few_parts() {
        let url = format!("void://open/{WS_ID}/@100");
        assert_eq!(parse(&url).unwrap_err(), DeepLinkError::InvalidCoordinates);
    }

    #[test]
    fn reject_coordinates_too_many_parts() {
        let url = format!("void://open/{WS_ID}/@1,2,3,4");
        assert_eq!(parse(&url).unwrap_err(), DeepLinkError::InvalidCoordinates);
    }
}
