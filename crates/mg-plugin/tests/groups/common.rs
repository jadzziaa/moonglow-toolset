#![allow(dead_code)]

use std::cell::{Cell, RefCell};
use std::collections::VecDeque;

use mg_plugin::{Answer, Host, Level, Question};

/// Configurable plugin [`Host`] shared across the grouped integration tests.
#[derive(Default)]
pub(crate) struct TestHost {
    pub(crate) log: RefCell<Vec<(Level, String)>>,
    pub(crate) answers: RefCell<VecDeque<Option<Answer>>>,
    pub(crate) asked: RefCell<Vec<Question>>,
    pub(crate) interactive: Cell<bool>,
    pub(crate) fill_page_defaults: bool,
    /// Looks at `cancelled` left before it says yes (`None`: never).
    pub(crate) cancel_after: Cell<Option<u32>>,
}

impl TestHost {
    pub(crate) fn with_page_defaults() -> Self {
        Self { fill_page_defaults: true, ..Default::default() }
    }

    pub(crate) fn log_texts(&self) -> Vec<String> {
        self.log.borrow().iter().map(|(_, s)| s.clone()).collect()
    }

    pub(crate) fn take_log_text(&self, index: usize) -> String {
        self.log.borrow_mut().remove(index).1
    }
}

impl Host for TestHost {
    fn log(&self, level: Level, text: &str) {
        self.log.borrow_mut().push((level, text.to_string()));
    }

    fn cancelled(&self) -> bool {
        match self.cancel_after.get() {
            None => false,
            Some(0) => true,
            Some(n) => {
                self.cancel_after.set(Some(n - 1));
                false
            }
        }
    }

    fn interactive(&self) -> bool {
        self.interactive.get()
    }

    fn ask(&self, question: &Question) -> Option<Answer> {
        self.asked.borrow_mut().push(question.clone());
        let answer = self.answers.borrow_mut().pop_front().flatten();
        if !self.fill_page_defaults {
            return answer;
        }
        // A page's fields not given are filled with their defaults, as a dialog answers.
        match (question, answer) {
            (Question::Page(p), Some(Answer::Page { mut values, back })) => {
                for f in &p.fields {
                    values.entry(f.id.clone()).or_insert_with(|| f.kind.default_value());
                }
                Some(Answer::Page { values, back })
            }
            (_, answer) => answer,
        }
    }
}
