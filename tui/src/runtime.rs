// SPDX-FileCopyrightText: Copyright © 2020-2023 Serpent OS Developers
//
// SPDX-License-Identifier: MPL-2.0

use std::{
    io::{stdout, Result},
    time::Duration,
};

use futures::{stream, FutureExt, StreamExt};
use ratatui::{
    prelude::CrosstermBackend,
    text::Line,
    widgets::{Paragraph, Widget},
    TerminalOptions, Viewport,
};
use tokio::{runtime, signal::ctrl_c, sync::mpsc, task, time};
use tokio_stream::wrappers::IntervalStream;

use crate::Program;

/// Run the TUI application within the async runtime and handle all
/// events automatically, including rendering and signals.
pub fn run<P: Program, T: Send>(
    mut program: P,
    f: impl FnOnce(Handle<P::Message>) -> T + Send + Sync + 'static,
) -> Result<T>
where
    P::Message: Send + 'static,
    T: 'static,
{
    let rt = runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    rt.block_on(async move {
        // Setup terminal
        let mut terminal = ratatui::Terminal::with_options(
            CrosstermBackend::new(stdout()),
            TerminalOptions {
                viewport: Viewport::Inline(P::LINES),
            },
        )?;

        // Draw initial view
        terminal.draw(|frame| {
            program.draw(frame);
        })?;

        // Setup channel
        let (sender, mut receiver) = mpsc::unbounded_channel();

        // We can receive render event or finished status
        enum Input<T> {
            Render,
            Finished(T),
            Term,
        }

        // Run task
        let mut run = task::spawn_blocking(move || f(Handle { sender }))
            .map(Input::Finished)
            .into_stream();
        // Ctrl c task
        let mut ctrl_c = ctrl_c().map(|_| Input::Term).into_stream().boxed();
        // Rerender @ 60fps
        let mut interval = IntervalStream::new(time::interval(Duration::from_millis(1000 / 60)))
            .map(|_| Input::Render);

        loop {
            // Get next input
            let input = stream::select(&mut run, stream::select(&mut ctrl_c, &mut interval))
                .next()
                .await
                .unwrap();

            match input {
                Input::Render => {
                    let mut print = vec![];

                    while let Ok(event) = receiver.try_recv() {
                        match event {
                            Event::Message(message) => program.update(message),
                            Event::Print(content) => print.push(content),
                        }
                    }

                    if !print.is_empty() {
                        let lines = print
                            .iter()
                            .flat_map(|content| content.lines())
                            .collect::<Vec<_>>();
                        let num_lines = lines.len();
                        let paragraph =
                            Paragraph::new(lines.into_iter().map(Line::from).collect::<Vec<_>>());

                        terminal.insert_before(num_lines as u16, |buf| {
                            paragraph.render(buf.area, buf)
                        })?;
                    }

                    terminal.draw(|frame| program.draw(frame))?;
                }
                Input::Finished(handle) => {
                    let ret = handle?;

                    terminal.show_cursor()?;
                    terminal.clear()?;

                    return Ok(ret);
                }
                Input::Term => {
                    terminal.show_cursor()?;
                    terminal.clear()?;
                    std::process::exit(0);
                }
            }
        }
    })
}

pub struct Handle<Message> {
    sender: mpsc::UnboundedSender<Event<Message>>,
}

impl<Message> Clone for Handle<Message> {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
        }
    }
}

impl<Message> Handle<Message> {
    pub fn print(&mut self, content: String) {
        let _ = self.sender.send(Event::Print(content));
    }

    pub fn update(&mut self, message: Message) {
        let _ = self.sender.send(Event::Message(message));
    }
}

pub enum Event<Message> {
    Message(Message),
    Print(String),
}
