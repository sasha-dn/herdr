use bytes::Bytes;

use crate::api::schema::{
    AgentChildrenParams, AgentInfo, AgentRenameParams, AgentSendParams, AgentSetParentParams,
    AgentStartParams, AgentTarget, PaneReadResult, ReadFormat, ReadSource, ResponseResult,
};
use crate::app::App;

use super::responses::{encode_error, encode_error_body, encode_success};

impl App {
    pub(super) fn handle_agent_list(&mut self, id: String) -> String {
        encode_success(
            id,
            ResponseResult::AgentList {
                agents: self.collect_agent_infos(),
            },
        )
    }

    pub(super) fn handle_agent_get(&mut self, id: String, target: AgentTarget) -> String {
        let agent = match self.agent_info_for_target(&target.target) {
            Ok(agent) => agent,
            Err(err) => return encode_error_body(id, self.agent_target_error_body(err)),
        };

        encode_success(id, ResponseResult::AgentInfo { agent })
    }

    pub(super) fn handle_agent_children(
        &mut self,
        id: String,
        params: AgentChildrenParams,
    ) -> String {
        let agent = match self.agent_info_for_target(&params.target) {
            Ok(agent) => agent,
            Err(err) => return encode_error_body(id, self.agent_target_error_body(err)),
        };

        let all = self.collect_agent_infos();
        let agents = collect_agent_children(&all, &agent.pane_id, params.recursive);
        encode_success(id, ResponseResult::AgentList { agents })
    }

    pub(super) fn handle_agent_focus(&mut self, id: String, target: AgentTarget) -> String {
        let agent = match self.focus_agent_target(&target.target) {
            Ok(agent) => agent,
            Err(err) => return encode_error_body(id, self.agent_target_error_body(err)),
        };

        encode_success(id, ResponseResult::AgentInfo { agent })
    }

    pub(super) fn handle_agent_rename(&mut self, id: String, params: AgentRenameParams) -> String {
        let agent = match self.rename_agent_target(&params.target, params.name) {
            Ok(agent) => agent,
            Err(err) => return encode_error_body(id, self.agent_rename_error_body(err)),
        };

        encode_success(id, ResponseResult::AgentInfo { agent })
    }

    pub(super) fn handle_agent_set_parent(
        &mut self,
        id: String,
        params: AgentSetParentParams,
    ) -> String {
        let agent = match self.set_agent_parent(&params.target, &params.parent) {
            Ok(agent) => agent,
            Err(err) => return encode_error_body(id, self.agent_set_parent_error_body(err)),
        };

        encode_success(id, ResponseResult::AgentInfo { agent })
    }

    pub(super) fn handle_agent_start(&mut self, id: String, params: AgentStartParams) -> String {
        let extra_env = match super::env::normalize_launch_env(params.env.clone()) {
            Ok(env) => env,
            Err((code, message)) => return encode_error(id, &code, message),
        };
        let (agent, argv) = match self.start_agent(params, extra_env) {
            Ok(started) => started,
            Err(err) => return encode_error_body(id, self.agent_start_error_body(err)),
        };

        encode_success(id, ResponseResult::AgentStarted { agent, argv })
    }

    pub(super) fn handle_agent_read(
        &mut self,
        id: String,
        params: crate::api::schema::AgentReadParams,
    ) -> String {
        let resolved = match self.resolve_terminal_target(&params.target) {
            Ok(resolved) => resolved,
            Err(err) => return encode_error_body(id, self.agent_target_error_body(err)),
        };
        let Some((pane, workspace_id)) = self.lookup_runtime(resolved.ws_idx, resolved.pane_id)
        else {
            return agent_not_found(id, &params.target);
        };
        let requested_lines = params.lines.unwrap_or(80).min(1000) as usize;
        let text = match params.format {
            ReadFormat::Text => match params.source {
                ReadSource::Visible => pane.visible_text(),
                ReadSource::Recent => pane.recent_text(requested_lines),
                ReadSource::RecentUnwrapped => pane.recent_unwrapped_text(requested_lines),
                ReadSource::Detection => pane.detection_text(),
            },
            ReadFormat::Ansi => match params.source {
                ReadSource::Visible => pane.visible_ansi(),
                ReadSource::Recent => pane.recent_ansi(requested_lines),
                ReadSource::RecentUnwrapped => pane.recent_unwrapped_ansi(requested_lines),
                ReadSource::Detection => pane.detection_text(),
            },
        };

        encode_success(
            id,
            ResponseResult::PaneRead {
                read: PaneReadResult {
                    pane_id: self
                        .public_pane_id(resolved.ws_idx, resolved.pane_id)
                        .unwrap_or_else(|| params.target.clone()),
                    workspace_id,
                    tab_id: self
                        .public_tab_id(resolved.ws_idx, resolved.tab_idx)
                        .unwrap(),
                    source: params.source,
                    format: params.format,
                    text,
                    revision: 0,
                    truncated: false,
                },
            },
        )
    }

    pub(super) fn handle_agent_explain(&mut self, id: String, target: AgentTarget) -> String {
        let resolved = match self.resolve_terminal_target(&target.target) {
            Ok(resolved) => resolved,
            Err(err) => return encode_error_body(id, self.agent_target_error_body(err)),
        };
        let Some((pane, _workspace_id)) = self.lookup_runtime(resolved.ws_idx, resolved.pane_id)
        else {
            return agent_not_found(id, &target.target);
        };
        let Some(terminal_id) = self
            .state
            .workspaces
            .get(resolved.ws_idx)
            .and_then(|workspace| workspace.terminal_id(resolved.pane_id))
        else {
            return agent_not_found(id, &target.target);
        };
        let Some(terminal) = self.state.terminals.get(terminal_id) else {
            return agent_not_found(id, &target.target);
        };
        if terminal.full_lifecycle_hook_authority_active() {
            let explain = serde_json::json!({
                "agent": terminal.effective_agent_label().unwrap_or("unknown"),
                "state": crate::detect::manifest::agent_state_label(terminal.state),
                "manifest_source": null,
                "manifest_version": null,
                "cached_remote_version": null,
                "local_override_shadowing_remote": false,
                "remote_update_status": null,
                "remote_update_error": null,
                "matched_rule": null,
                "visible_idle": false,
                "visible_blocker": false,
                "visible_working": false,
                "screen_detection_skipped": true,
                "screen_detection_skip_reason": "full_lifecycle_hook_authority",
                "skip_state_update": false,
                "skipped_update_reason": null,
                "fallback_reason": null,
                "warning": null,
                "evaluated_rules": [],
            });
            return encode_success(id, ResponseResult::AgentExplain { explain });
        }
        let Some(agent) = terminal.effective_known_agent().or(terminal.detected_agent) else {
            return encode_error(
                id,
                "agent_explain_unavailable",
                format!(
                    "agent target {} does not have a detected agent label",
                    target.target
                ),
            );
        };

        let screen = pane.detection_text();
        let osc_title = pane.agent_osc_title();
        let osc_progress = pane.agent_osc_progress();
        let explain = crate::detect::manifest::explain_with_input(
            agent,
            crate::detect::manifest::DetectionInput {
                screen: &screen,
                osc_title: &osc_title,
                osc_progress: &osc_progress,
            },
        );
        let value = crate::detect::manifest::explain_to_json_value(&explain);

        encode_success(id, ResponseResult::AgentExplain { explain: value })
    }

    pub(super) fn handle_agent_send(&mut self, id: String, params: AgentSendParams) -> String {
        let resolved = match self.resolve_terminal_target(&params.target) {
            Ok(resolved) => resolved,
            Err(err) => return encode_error_body(id, self.agent_target_error_body(err)),
        };
        let Some(runtime) = self.lookup_runtime_sender(resolved.ws_idx, resolved.pane_id) else {
            return agent_not_found(id, &params.target);
        };
        if let Err(err) = runtime.try_send_bytes(Bytes::from(params.text)) {
            return encode_error(id, "agent_send_failed", err.to_string());
        }

        encode_success(id, ResponseResult::Ok {})
    }
}

fn agent_not_found(id: String, target: &str) -> String {
    encode_error(
        id,
        "agent_not_found",
        format!("agent target {target} not found"),
    )
}

/// Collect the agents whose parent is `parent_pane_id`. With `recursive`, the
/// whole descendant subtree is returned in preorder (each child immediately
/// followed by its own subtree); otherwise only the direct children. Children
/// keep the order they appear in `all`. A visited set guards against malformed
/// cyclic parent links.
fn collect_agent_children(
    all: &[AgentInfo],
    parent_pane_id: &str,
    recursive: bool,
) -> Vec<AgentInfo> {
    let mut out = Vec::new();
    let mut visited = std::collections::HashSet::new();
    append_agent_children(all, parent_pane_id, recursive, &mut out, &mut visited);
    out
}

fn append_agent_children(
    all: &[AgentInfo],
    parent_pane_id: &str,
    recursive: bool,
    out: &mut Vec<AgentInfo>,
    visited: &mut std::collections::HashSet<String>,
) {
    for agent in all {
        if agent.parent.as_deref() != Some(parent_pane_id) {
            continue;
        }
        if !visited.insert(agent.pane_id.clone()) {
            continue;
        }
        out.push(agent.clone());
        if recursive {
            append_agent_children(all, &agent.pane_id, recursive, out, visited);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::collect_agent_children;
    use crate::api::schema::{AgentInfo, AgentStatus};

    fn agent(pane_id: &str, parent: Option<&str>) -> AgentInfo {
        AgentInfo {
            terminal_id: format!("term_{pane_id}"),
            name: None,
            agent: None,
            title: None,
            display_agent: None,
            agent_status: AgentStatus::Unknown,
            screen_detection_skipped: false,
            custom_status: None,
            state_labels: std::collections::HashMap::new(),
            agent_session: None,
            workspace_id: "w1".into(),
            tab_id: "w1:1".into(),
            pane_id: pane_id.into(),
            focused: false,
            cwd: None,
            foreground_cwd: None,
            parent: parent.map(|value| value.into()),
            revision: 0,
        }
    }

    fn pane_ids(agents: &[AgentInfo]) -> Vec<&str> {
        agents.iter().map(|agent| agent.pane_id.as_str()).collect()
    }

    // Tree: parent p1 has children p2, p3; p2 has grandchild p4; p5 is a root.
    fn tree() -> Vec<AgentInfo> {
        vec![
            agent("w1:p1", None),
            agent("w1:p2", Some("w1:p1")),
            agent("w1:p4", Some("w1:p2")),
            agent("w1:p3", Some("w1:p1")),
            agent("w1:p5", None),
        ]
    }

    #[test]
    fn direct_children_only_in_source_order() {
        let all = tree();
        let children = collect_agent_children(&all, "w1:p1", false);
        assert_eq!(pane_ids(&children), vec!["w1:p2", "w1:p3"]);
    }

    #[test]
    fn recursive_children_are_preorder() {
        let all = tree();
        let children = collect_agent_children(&all, "w1:p1", true);
        // p2, then p2's subtree (p4), then p3.
        assert_eq!(pane_ids(&children), vec!["w1:p2", "w1:p4", "w1:p3"]);
    }

    #[test]
    fn leaf_and_root_without_children_return_empty() {
        let all = tree();
        assert!(collect_agent_children(&all, "w1:p4", true).is_empty());
        assert!(collect_agent_children(&all, "w1:p5", true).is_empty());
    }

    #[test]
    fn cyclic_parent_links_do_not_recurse_forever() {
        // p1 -> p2 -> p1 forms a cycle. Collection must terminate, visiting each
        // node at most once (p2 as p1's child, then p1 as p2's child).
        let all = vec![agent("w1:p1", Some("w1:p2")), agent("w1:p2", Some("w1:p1"))];
        let children = collect_agent_children(&all, "w1:p1", true);
        assert_eq!(pane_ids(&children), vec!["w1:p2", "w1:p1"]);
    }
}
