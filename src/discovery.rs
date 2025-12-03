use std::process::Command;

use anyhow::Result;

/// Information about a discovered container or pod
#[derive(Debug, Clone)]
pub struct DiscoveredSource {
    /// Name of the container/pod
    pub name: String,
    /// Type of source (for display)
    #[allow(dead_code)]
    pub source_type: SourceType,
    /// Current status (running, stopped, etc.)
    pub status: String,
    /// Extra info (image name, containers, etc.)
    pub extra: Option<String>,
    /// Namespace (for K8s pods)
    pub namespace: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceType {
    Docker,
    K8s,
}

impl std::fmt::Display for SourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SourceType::Docker => write!(f, "Docker"),
            SourceType::K8s => write!(f, "K8s"),
        }
    }
}

/// Discover running Docker containers
pub fn discover_docker_containers() -> Result<Vec<DiscoveredSource>> {
    let output = Command::new("docker")
        .args(["ps", "--format", "{{.Names}}\t{{.Status}}\t{{.Image}}"])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("docker ps failed: {}", stderr);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let sources = stdout
        .lines()
        .filter(|line| !line.is_empty())
        .filter_map(|line| {
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() >= 2 {
                Some(DiscoveredSource {
                    name: parts[0].to_string(),
                    source_type: SourceType::Docker,
                    status: parts[1].to_string(),
                    extra: parts.get(2).map(|s| s.to_string()),
                    namespace: None, // Docker doesn't have namespaces
                })
            } else {
                None
            }
        })
        .collect();

    Ok(sources)
}

/// Discover Kubernetes pods
pub fn discover_k8s_pods(namespace: Option<&str>) -> Result<Vec<DiscoveredSource>> {
    let mut cmd = Command::new("kubectl");

    // Include NAMESPACE in output when querying all namespaces
    let all_namespaces = namespace.is_none();
    if all_namespaces {
        cmd.args([
            "get",
            "pods",
            "--all-namespaces",
            "-o",
            "custom-columns=NAMESPACE:.metadata.namespace,NAME:.metadata.name,STATUS:.status.phase,CONTAINERS:.spec.containers[*].name",
        ]);
    } else {
        cmd.args([
            "get",
            "pods",
            "-n",
            namespace.unwrap(),
            "-o",
            "custom-columns=NAME:.metadata.name,STATUS:.status.phase,CONTAINERS:.spec.containers[*].name",
        ]);
    }

    let output = cmd.output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("kubectl get pods failed: {}", stderr);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let sources = stdout
        .lines()
        .skip(1) // Skip header row
        .filter(|line| !line.is_empty())
        .filter_map(|line| {
            let parts: Vec<&str> = line.split_whitespace().collect();

            if all_namespaces {
                // Format: NAMESPACE NAME STATUS CONTAINERS
                if parts.len() >= 3 {
                    Some(DiscoveredSource {
                        namespace: Some(parts[0].to_string()),
                        name: parts[1].to_string(),
                        source_type: SourceType::K8s,
                        status: parts[2].to_string(),
                        extra: parts.get(3).map(|s| s.to_string()),
                    })
                } else {
                    None
                }
            } else {
                // Format: NAME STATUS CONTAINERS
                if parts.len() >= 2 {
                    Some(DiscoveredSource {
                        namespace: namespace.map(|s| s.to_string()),
                        name: parts[0].to_string(),
                        source_type: SourceType::K8s,
                        status: parts[1].to_string(),
                        extra: parts.get(2).map(|s| s.to_string()),
                    })
                } else {
                    None
                }
            }
        })
        .collect();

    Ok(sources)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_source_type_display() {
        assert_eq!(format!("{}", SourceType::Docker), "Docker");
        assert_eq!(format!("{}", SourceType::K8s), "K8s");
    }

    #[test]
    fn test_discovered_source_clone() {
        let source = DiscoveredSource {
            name: "test".to_string(),
            source_type: SourceType::Docker,
            status: "running".to_string(),
            extra: Some("nginx:latest".to_string()),
            namespace: None,
        };
        let cloned = source.clone();
        assert_eq!(cloned.name, "test");
        assert_eq!(cloned.source_type, SourceType::Docker);
    }

    #[test]
    fn test_discovered_source_with_namespace() {
        let source = DiscoveredSource {
            name: "my-pod".to_string(),
            source_type: SourceType::K8s,
            status: "Running".to_string(),
            extra: Some("nginx".to_string()),
            namespace: Some("production".to_string()),
        };
        assert_eq!(source.namespace, Some("production".to_string()));
    }
}
