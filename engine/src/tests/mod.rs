use std::{
    collections::HashMap,
    ops::Deref,
    sync::{Arc, Mutex},
};

use action_orc_core::*;
use action_orc_macros::*;
use test_macros::*;

use crate::*;

mod graph;
mod orchestrator;
mod reactor;

mod cmd {
    use super::*;

    pub fn resolve(pivot: NodeId) -> NodeCommand {
        NodeCommand {
            schedule_directive: ScheduleDirective::Resolve { pivot },
        }
    }

    pub fn advance(pivot: NodeId, target: NodeId) -> NodeCommand {
        NodeCommand {
            schedule_directive: ScheduleDirective::Advance { pivot, target },
        }
    }

    pub fn backtrack() -> NodeCommand {
        NodeCommand {
            schedule_directive: ScheduleDirective::Backtrack,
        }
    }
}

type Result = testresult::TestResult;

fn drain_commands(reactor: &mut Reactor, commands: Arc<Mutex<Vec<NodeCommand>>>) -> Result {
    loop {
        let pending: Vec<NodeCommand> = {
            let mut guard = commands.lock()?;
            std::mem::take(&mut *guard)
        };

        if pending.is_empty() {
            break;
        }

        for command in pending {
            reactor.process(command)?;
        }
    }
    Ok(())
}

type Log = Arc<Mutex<Vec<(NodeId, NodeStatus)>>>;
type Commands = Arc<Mutex<Vec<NodeCommand>>>;

struct TestLogger {
    log: Log,
    commands: Commands,
}

pub struct TestCtx {
    log: Log,
    commands: Commands,
}

impl TestCtx {
    #[track_caller]
    pub fn record(&self, id: NodeId, status: NodeStatus) {
        self.log.lock().unwrap().push((id, status));
    }

    #[track_caller]
    pub fn queue_cmd(&self, cmd: NodeCommand) {
        self.commands.lock().unwrap().push(cmd);
    }
}

impl TestLogger {
    fn new() -> Self {
        Self {
            log: Arc::new(Mutex::new(Vec::new())),
            commands: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn with_ctx(
        &self,
        handler: impl Fn(NodeId, NodeStatus, &TestCtx) + Send + Sync + 'static,
    ) -> impl Fn(NodeId, NodeStatus) + Send + Sync + 'static {
        let ctx = TestCtx {
            log: self.log.clone(),
            commands: self.commands.clone(),
        };

        move |id, status| handler(id, status, &ctx)
    }

    #[track_caller]
    fn check_log(&self, expected: Vec<(NodeId, NodeStatus)>) {
        let actual = self.log.lock().unwrap().clone();
        assert_eq!(actual, expected);
    }
}

pub struct NodeMap(HashMap<&'static str, NodeId>);

impl Deref for NodeMap {
    type Target = HashMap<&'static str, NodeId>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> std::ops::Index<T> for NodeMap
where
    T: Into<&'static str>,
{
    type Output = NodeId;

    #[track_caller]
    fn index(&self, tag: T) -> &Self::Output {
        let key = tag.into();
        self.0.get(key).unwrap()
    }
}

impl Reactor {
    fn map_nodes(&self) -> NodeMap {
        let mut s2i = HashMap::new();
        for (id, meta) in self.node_meta() {
            s2i.insert(meta.type_name(), id);
        }
        NodeMap(s2i)
    }
}

fn no_op_listener(_: usize, _: NodeStatus) {}

mod test_macros {
    macro_rules! mock_listeners {
        ($reactor:expr, $listener:expr, $($tag:ident),* $(,)?) => {
            let shared_listener = std::sync::Arc::new($listener);
            $(
                $reactor.listen_for(
                    std::any::TypeId::of::<$tag>(),
                    shared_listener.clone()
                )?;
            )*
        };
    }
    pub(crate) use mock_listeners;
}
