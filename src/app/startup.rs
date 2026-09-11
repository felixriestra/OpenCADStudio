use super::{ModalKind, OpenCADStudio};

impl OpenCADStudio {
    pub(super) fn queue_startup_prompts(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        if !self.default_assoc_prompted {
            self.pending_startup_modals
                .push_back(ModalKind::AssocPrompt);
        }
        self.show_next_startup_modal();
    }

    pub(super) fn show_next_startup_modal(&mut self) {
        if self.active_modal.is_none() && self.opening.is_none() && self.pending_opens.is_empty() {
            if let Some(&kind) = self.pending_startup_modals.front() {
                self.active_modal = Some(kind);
                self.reset_modal_geometry();
            }
        }
    }

    pub(super) fn mark_startup_modal_shown(&mut self) {
        if self.active_modal.is_some()
            && self.active_modal.as_ref() == self.pending_startup_modals.front()
        {
            self.pending_startup_modals.pop_front();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{config::AppConfig, Message};

    #[test]
    fn startup_dialogs_wait_their_turn() {
        let mut app = OpenCADStudio::new_for_test();
        app.apply_config(AppConfig::default());
        app.queue_startup_prompts();
        assert_eq!(app.active_modal, Some(ModalKind::AssocPrompt));

        let _ = app.update(Message::UpdateCheckResult(Some(
            crate::io::update_check::UpdateInfo {
                version: "next-release".to_string(),
                body: String::new(),
            },
        )));
        assert_eq!(app.active_modal, Some(ModalKind::AssocPrompt));

        let _ = app.update(Message::AssocPromptNo);
        assert_eq!(app.active_modal, Some(ModalKind::UpdateNotice));
        let _ = app.update(Message::UpdateNoticeClose);
        assert!(app.active_modal.is_none());
        assert!(app.pending_startup_modals.is_empty());
    }
}
