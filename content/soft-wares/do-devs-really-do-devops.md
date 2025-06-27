+++
title = "Do Devs Really Do DevOps in your Org?"
template = "page.html"
weight = 0
draft = false
date = 2025-06-26
[extra]
updated = 2025-06-27
description = "A critical look at how DevOps is implemented in organizations, examining the gap between the promise of shift-left practices and the reality of developers' access to infrastructure"
+++

## Do Devs Really Do DevOps in your Org?

Recently, I learned the more formal definition of shift-right and shift-left in terms of Agile DevOps. For a brief refresher and for brevity it goes a little something like this:

- Shift Right -> Validation and testing in production
- Shift Left -> Validation and testing are before production

Now that's kind of the intended definition, and it makes perfect sense. In fact, I would probably go, "hell yea, this is just smart." I naturally subscribe to the XP (Extreme Programming) subset of Agile, and that generally means I just pump out tiny slices rapidly that are often not a complete feature. Think of it like writing a book chapter by chapter and having your editor review it as you go. This kind of process means you will miss some things on the first pass but spend less time on __discovery__. Not advocating just calling out the causal nature of this decision. So there is a lot of refactoring and revelation through the process that this creates.

Generally, shift left proposes some big claims; the stinkiest are the following: [ref](https://www.dynatrace.com/news/blog/what-is-shift-left-and-what-is-shift-right/#the-benefits-of-shift-left)

- Reduces Cost
- Improves Collaboration
- Faster time to market

Holy cow, sign me up, I want _cheaper_, _faster_, and _better_. Even if the claim violates the mere existence of the _good_, _fast_, and _cheap_ love triangle. We all smell nothing, but poking fun at the top keyword buyer shill article wrapped as a blog post isn't the goal here. I wanna bring this into focus of practicals and current experiences.

Here is my experience with shift left devops in the wild. As a developer, I am giving access to a CI runner that can execute terraform or cloudformation, doesn't matter. I am given some tools that might add some constraints to that process like a terraform wrapper or a set of CI templates. I am then told I can just build whatever I want. Except:

- I have no way to interact directly with the terraform state.
- I can't view resources in the cloud providers console, and I cannot manage IAM roles/policies.

What I have been granted is the illusion that I can self-service and a new stack of problems to solve through a fog. Because, I can ask for support, but it will be through a ticketing system, and the resolution will take weeks.

While I am skilled in devops, I would say that 80% of my peers are not, and thinking about it, the condition I have described is simply, go use terraform but never actually run terraform. All operations must be performed through an environmental suit. Let's revisit our targets, unrealizable as they may seem. Have we reduced cost? In some ways, yes, by distributing the workload we have reduced the need for specialized staff, we can argue that most devops are routine and probably cookie cutter, so having a group oversee the orgs work streams is a better spend. Does it improve collaboration, probably not. As I have been the member of now a SecOps team, run a DevOps team, lead a L3 Support team, and spent the rest of my career in mines as a developer. When you create a centralized management team, you have a choice, they can directly collaborate, providing deep value and insight as they touch the people they support, or you can make them the slaves of a ticketing system. Since its rather difficult to account for performance and costs associated with staffing without any figures to back them up, you will end up with a ticketing system, probably. Let's be honest, that's not, "Improves Collabortion," that's, "Sets up a call center." So you put some very talented people place them in the complaints department and say, drink from the firehose, k thx bye. This is getting too dark, let's circle back about cost again, so your devs are reaching out to your devops team for support through your ticketing system, and you can track your MTTR as 3 days. Wow, Kudos, we are doing business, good business actions! But had we honestly shifted left, the developer would have probably solved the issue themself had there been some trust and access. It would have taken a couple of hours and possibly a message in a chat channel for a code review. I get it this is argumentative but its also a generalized understatement. MTTR isn't 3 days, it's 3 weeks, and the dev would have solved it in days, not hours, but it's about ratios. That, of course, puts a finger on the last target of delivery speed.

I kinda see this like AI for devs these days, giving away our ownership feels bad because for many of us it means we might be less special. It might mean that critical thinking and planning are the real skills, not butts in seats. Of course this is about devops, not LLMs and code gen, so what's the next step?

First off, let's make the sharing of responsibility for devops not a chore. I need infrastructure and devops people to do the good work, help me pick the right tools, be experts in the cloud or servers. I see that as the divider, __knowledge not access__. What you want from your developers is to be able to find resources and use tools; sometimes people and sometimes documentation. Then you want them to be able to evaluate those things before they make their way to production. This means taking away the training wheels, devops teams produce products, not interfaces. There was a time when terraform was the new kid on the block that devops teams provided modules to isolate the patterns they want to repeat. I know this is how I did it when I ran my first DevOps team. We didn't kind any of the sausage and we provided support like any _Open Source_ project would. There was documentation and READMEs, along with tools and tutorials. Most importantly, there was access, developers could run terraform locally, create infra using their developer accounts, and submit pull requests for our modules. Better yet, they could read our modules. We did a lot of the first people on the scene to a new concern; once vetted, it was normalized for repeat use. We saw ourselves as the caretakers, with a motto of "yes and." We used a ticketing system but only internally if something was more than just a conversational solution. We took notes and turned those conversations into FAQs. We did a lot of work in chat, we relied on the fact that our company chat was searchable. We keep discussions in public channels as a backup body of knowledge. We trained devs to talk devops, and what we discovered is or devs loved to learn. Plenty of the time we didn't even need to respond to a chat request, since it was probable someone else had already encountered the issue they would speak up.

I know it sounds like, I have been poo-pooing the farcical benefits of shift left, and I surely am. I just want to remind you when we talk about money and timelines in polite company its considered gauche. Although, we can spend 10 minutes of a 30-minute meeting worried about efficiencies with 6 other humans. That's an hour spent bub, by the way. Instead, we _focus on the people and the problem_, _not the problem is the people_. All I am asking is for you to consider this famous quote, "The more you tighten your grip, the more star systems will slip through your fingers." There are orgs that get this kinda thing right and it's not just devops. Its always good to identify if you are in one of those orgs and what you can do to change it. If you wanna shift left, shift left. Otherwise, hire some more smart people and adopt waterfall for your projects, it works great, honestly. If you can't trust your devs to mind the cashbox with regard to infrastructure access, it means you haven't done the upfront work to position your DevOps team as a sheppard of their craft.
