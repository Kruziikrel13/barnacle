# Contributing to Barnacle GUI

## `Message` vs. `Event`

`Message` enums are for component-local communication, while `Event` enums are for communicating with the parent component.

You might wonder "why can't the parent component just match on the child's `Message` in if the child wants it to perform an action?". This is because it makes it unclear that child is handing off responsibility to the parent. For instance:

```rust
mod child {
    pub enum Message {
        SomeLocalAction
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
           SomeLocalAction => Task::none() 
        }
    }
}

pub enum Message {
    Child(child::Message)
}

pub fn update(&mut self, message: Message) -> Task<Message> {
    match message {
        Child(message) => match {
            child::Message::SomeLocalAction => println("Do something"),
        }
    }
}
```

As you can see it's very unclear here that the child is handing off responsibility. It looks like it's just doing nothing. If we instead send an event (e.g `Event::ParentAction`), then it's clear what the child component wants.
