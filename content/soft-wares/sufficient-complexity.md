+++
title = "Sufficient Complexity"
template = "page.html"
weight = 0
draft = false
date = 2025-07-01
updated = 2025-07-01
[extra]
desc = "Exploring the concept of sufficient complexity in software design and how to create maintainable, focused modules that avoid common pitfalls of over-engineering"
keywords = "software complexity, modules, interfaces, over-engineering, software design, dependencies, architecture"
discussion_number = 42
discussion_url = "https://github.com/orgs/developmeh/discussions/42"
+++

## Sufficient Complexity and Pipe Herding

I still think a lot of what I do day to day in software aligns well with plumbing. DevOps is like warehouse work, and Product is herding. Before the last batch of product engineers, pipe herders, I recall much conversing about things like pragmatism and simplicity. When you think about the pipes in your home, you hope they aren't too complicated. Complication in pipes leads to pinholes, leaks, and sudden noises from the dark places we dare not look. Plastered over and expected to last the long haul, we forget exactly where each one is until we wanna make an addition, drill a hole, or something goes terribly wrong. Software is often like this too, long forgotten, and sometimes completely unused sections live in the shadows. I really thought this was rather boring in my younger years and creates an interesting condition in the enterprise around scope. Too much scope, and we get exactly what we think we want. Too little scope and we end up with too many pipes. I still believe that much of this was dogmatic rhetoric. A book makes the rounds and is praised, others consume it and take it as cannon, producing an effect that is the same as not knowing, over-knowing. Now, with much of that kind of nonsense falling out of style, we wonder why we can't keep our software from crashing. To be crystal I am not saying that due to a failure to care about craft is the reason this is happening. Craft, for better or worse, is its one kind of over-knowing, consider if through rigorous focus and process one can eliminate mistakes or bad design. I argue it's just a matter of speed, things that go fast tend to have lower survivability. A generalization for sure but its better to relate this to change management over velocity in the mind.

So Sufficient Complexity is the mark where we can say something is done, not to be confused with finished. This feels impossible in the land of building products on the web, but I promise it is still achievable. It also doesn't matter if you are working with a monolith or a microservice, but it is about dependencies at the core. Step 1 is to eliminate the word _common_ from your vernacular, followed by _shared_. While they may look safe, these are traps, hear to eat your time and sanity. Just like we can have a perfect project layout like [here](devex/the-perfect-dev-env/), we can have just the right size of features in a box.

Now here is the most important lesson you will learn, __Everything is a File or Folder__ depending on your observable distance. This counts for how you organize your project in version control all the way to how you deploy your application containers. Folders and files are what matter and the relationships they play to each other. I feel like the oft overlooked power of all of this is the _interface_ or the _interaction pattern_. It gives us a fixed view of how something is to be consumed or constructed and provides the most meaning in relation to producing moderately stable software. Consider the following I have a folder full of functions on the left and a folder of consumers on the right, between them, I organize those functions into groups called interfaces. Once two interfaces share the same function, I have created a problem, one that is sometimes unsolvable but often avoidable.

<span style="" class="mermaid">
graph LR
    subgraph "Functions"
        F1[Function 1]
        F2[Function 2]
        F3[Function 3]
        F4[Function 4]
        F5[Function 5]
    end
    subgraph "Interfaces"
        I1[Interface A]
        I2[Interface B]
    end
    subgraph "Consumers"
        C1[Consumer X]
        C2[Consumer Y]
    end
    %% Good pattern - clean separation
    F1 --> I1
    F2 --> I1
    F3 --> I2
    F4 --> I2
    F5 --> I2
    I1 --> C1
    I2 --> C2
    %% Problem case - shared function
    F3 -.-> I1
    style F3 fill:#f96,stroke:#333
    style I1 stroke:#f00,stroke-width:2px
    style I2 stroke:#f00,stroke-width:2px
</span>

> The diagram above illustrates the concept. On the left, we have a collection of functions (Function 1-5). In the middle, we have interfaces (A and B) that group these functions. On the right, we have consumers (X and Y) that use these interfaces.
>
> The problem occurs when Function 3 is shared between Interface A and Interface B (shown by the dotted line). This creates coupling between the interfaces and can lead to issues when one interface needs to change but can't without affecting the other interface. This is why interface segregation is important - each interface should have a single, focused purpose with its own dedicated functions.

That's just if we share the same functions across two interfaces. Imagine how the rest of the internet works? So here is where your bugs are coming from most likely. To be clear, there is nothing you can do to avoid it, so lets get the doom and gloom out of the way. Code like pipes work the best when singularly focused. For example a pipe that feeds other pipes isn't a faucet line but a feed line. Maybe it started its life out going from the source to your bathroom faucet, and at some later point you installed a shower. At that point you created an interface, a physical one, and the nature of the pipe changed. At first it interfaced with the faucet and then that line fed an interface which interfaced with the new pipes, those interfaced with the shower and faucet individually. If we were then going to install a washing machine, (a beautiful European concept) in our bathroom, we might realize that the feed line in place doesn't meet the volume our washing machine needs. We will have to run a separate line for our washing machine.

We don't usually make the same decision with software, though, bits are very malleable, and our pipes are scalable with an injection of cash. I like to think of how to deal with coupling the same way I deal with pipes. If my needs cannot be met at the current interface, it's time for a new line maybe that's a new module or a new microservice and it might even copy some of the code from the existing pipe but it doesn't make a dependency on it. Long term we want to create solid permanent things. That are resistant to external change unless acted upon. I know this sounds like heresy and a lot of work but I promise its worth it. You will not end up with a bunch of duplicate code that matters. The things you copy will be boilerplate specific to the cause. The parts you don't copy are the items that you can depend on that don't require you to modify their interface.

Sounds hokey and more of a cry for "hey this way stinks, go do it this other way cause it's different." It's not a new concept, though, because this is the principle of module boundaries. I usually explain this to my team as not _Goals_ and _Non Goals_ but _Spiritual Goals_ if I wanted to draw a circle around a unit if code such that it produced no more and no less than it needed to and met the _spirit_ of its purpose that is what we build. Its not as hand wavy as it sounds, but it does require understanding the scope of the work completely which I'll admit is not something everyone can always do. Arguably its this need to navigate ambiguity which leads more to poor design than inexperience. But I like this more formal term of _Sufficient Complexity_ to make _Spiritual Goals_ less techo hippie. Allow us to continue, a module is sufficiently complex when it provides a new complete boundary of its context. Your ears might be itching because this sounds a lot like Domain Driven Design(DDD), and you would probably be right. But DDD is interested in pathways through a system and is a very top down kind of concept. I, on the overhand am proposing a bottoms up approach. Something that I might slot into Agile or XP where we don't know all the scope before we start, and that's both normal and ok. But as we discover complexity, we promote context boundaries instead of the shortest path to completion.

#### Examples

Let's explore a couple of simple examples, first the webapp-common lib and then the universal modal dialog.


__Webapp-common__
In __webapp-common__ as the title describes, we are going to configure a number of tools and dependencies that all our webapps share inside a single module. The first question we should ask, after we stop screaming because we successfully forgot the word _common_ from earlier, is does this module describe a clear boundary for behavior? __No__ not really. If you said yes, that's ok, you may even think the boundary is web apps. Still not wrong but not great either, because this exceeds __Sufficient Complexity__ how can I imagine this module every being done. Since we will have many web apps and they will have all kinds of responsibilities its likely not every web app will need all the functionality of the webapp-common. This introduces a risk of being a dumping ground of interdependent libraries that over time, event versioned, will slowly start to poison each other. Because these libraries are also commonly shared, this pollution will touch everything.

Whats the solution? Well its always about informing on the pattern through an interface. If this is a Java Springboot project, we would want to introduce bean configurations optional transitive dependencies. Check out this sample project [java-no-more-common-lib](https://github.com/developmeh/java-no-more-common-lib)

```bash
spring-gradle-example/
├── build.gradle                 # Root project build file with common configurations
├── settings.gradle              # Project settings file
├── jackson-module/              # Jackson module with baseline dependencies and configurations
│   ├── build.gradle             # Jackson module build file with Jackson dependencies
│   └── src/
│       └── main/
│           ├── java/
│           │   └── com/example/jackson/config/
│           │       └── JacksonConfig.java  # Auto-configured Jackson configuration
│           └── resources/
│               └── META-INF/
│                   └── spring.factories    # Auto-configuration registration
└── service-module/              # Service module that uses the jackson module
    ├── build.gradle             # Service build file that overrides Jackson versions
    └── src/
        └── main/
            └── java/
                └── com/example/service/
                    └── ServiceApplication.java  # Spring Boot application

```

> Here the solution is to avoid common and instead create building blocks, this works with maven or gradle but gradle is a little clearer. The only thing that __jackson-module__ exposes is a specific configuration for jackson and provides a baseline for the jackson version. For our implementation to be sufficient we can use that baseline or in this case override it with the version we want. Given the version we choose can meet the bean configuration this provides we can apply this as an interface to our web-app. I picked jackson here because they are pretty bad at SEMVER, and I often have features that work in one minor version and not in another due to poor planning or deprecation. The problem is my need to jackson is intermingled with a whole common library usually. I can, of course, override and qualify a new bean as primary but I would rather have a choice if I should include it first. So instead of having spring.factories load this bean, I can @Import it in my application. This way I can control what my application consumes from its library. While that would be foolish for such a simple dependency. There is a real case where jackson and a few other libraries are joined together in a serde (serialization/deserialization) module. Which has a slightly broader context but it supplies common configurations for our serde.

Regardless, the point here is we often see the _simplicity_ of a common library that sets up all our dependencies as a big win when we start a group of projects. It quickly becomes something of a pain point when too many live together without a spiritual goal or shared contextual binding to make it meaningful to include it. Also, setting up a new project is not where the time in development is lost. So making that to focus on speed is a false hope. We always want to make maintenance of a code base the easiest solution. But moreso we want to create a space where we don't have to touch a codebase for a long time because its __done__ and thus __Sufficiently Complex__.

__The Universal Modal__

So now lets switch to Javascript and React land, we have a fantastic pattern for module size in this ecosystem and we probably don't have some annoying common library mucking up our sanity. But, we also work in the more visual scope and that means less technical people can fail to understand the nature of our work. They kind of see it as "configuring the browser" and less of a formal data flow and user interaction platform. We have been asked to build a model that can act a little like a slide deck. Starting in one context and then on each step asking if there is another context and providing a new interaction on each slide. Honestly, this sounds pretty cool, and I bet many someones out there have tried to make something like this. The first question we should ask, after we stop screaming because we are building power point as a model, is does this module describe a clear boundary for behavior? Once again __No__ not really. If you said yes, that's ok, you may even think the boundary is a dynamic modal or an iframe. Still not wrong but not great either, because this exceeds __Sufficient Complexity__ how can I know all components and interactions a designer might want to sequence before we know they exist?

When we build general purpose components for the case of reuse we are falling into the trap of creating code that over-knows about its purpose. This is the poster child of the cat with 4 normal lets, and a human ear and arm jammed on there we all saw back in school to describe software in the wild. This code will never be done and will continue to acquire features and conditions until it becomes to complex to work with. It will also be a nightmare to test because while a flow of slides can be static in intention they are in fact dynamic and the synchronization of what a test can verify and what we will present will diverge.

This is very much the counter example of the former. Instead of worrying about known competing dependencies we are building something smarter than we are right now that can anticipate the changes of tomorrow with complex design. The time spent building such a tool will never pay off, yes we can make something new nearly instantly but we can't test it instantly. It will also be a constant source of bugs that will also be complicated to verify.

What do we do instead? Well focus on what is _Sufficiently Complex_. The next question we should be asking is, do we know what we want to build? __No__ doesn't sound like it. Every time the idea of building the solve everything module comes up just accept that it would be better to know what needs doing now. How we can codify a process that makes repetition require less discovery for the next person and build exactly what we need. I promise when you need to come back and adjust slide 3 of flow 1 you will be much happier that you build layout components so you have uniform styling and that just because you are changing the info link on slide 3 it doesn't automatically bump around all the info links on the other flows. Like all good poetry we need to start with a rhyme. Software is hard to rhyme at first and poets spend a lot of time with words before becoming poets. So an expert can create a successful general system but there is also a lot of bad poetry out there. We build similar components and keep an ear out for the rhymes. Each pass we make we reverberate those sounds until we have something that repeats. Essentially, you don't start with the poem you start with they rhyme and the theme. Which is at its core an interface we make with this kind of component.
