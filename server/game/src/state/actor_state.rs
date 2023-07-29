use std::sync::Arc;

use arc_swap::ArcSwapOption;

use crate::systems::Screen;
use crate::world::Character;
use crate::Error;

#[derive(Debug)]
pub struct ActorState {
    character: ArcSwapOption<Character>,
    screen: ArcSwapOption<Screen>,
}

#[async_trait::async_trait]
impl tq_network::ActorState for ActorState {
    fn init() -> Self {
        ActorState {
            character: Default::default(),
            screen: Default::default(),
        }
    }
}

impl ActorState {
    pub fn set_character(&self, character: Character) {
        self.character.store(Some(Arc::new(character)));
    }

    pub fn set_screen(&self, screen: Screen) {
        let screen = Arc::new(screen);
        self.screen.store(Some(screen.clone()));
        self.character().set_screen(screen);
    }

    pub fn character(&self) -> Arc<Character> {
        self.character.load().clone().expect("state is not empty")
    }

    pub fn try_character(&self) -> Result<Arc<Character>, Error> {
        self.character
            .load()
            .clone()
            .ok_or(Error::CharacterNotFound)
    }

    pub fn screen(&self) -> Arc<Screen> {
        self.screen.load().clone().expect("state is not empty")
    }

    pub fn try_screen(&self) -> Result<Arc<Screen>, Error> {
        self.screen.load().clone().ok_or(Error::ScreenNotFound)
    }
}
