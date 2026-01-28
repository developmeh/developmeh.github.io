+++
title = "The AI Diaries"
template = "page.html"
weight = 0
draft = false
slug = "ai-diaries"
date = 2026-01-19
updated = 2026-01-28
[extra]
desc = "Anecdotes and observations from working with AI coding tools - the hype, the reality, and the debt"
keywords = "AI, coding tools, developer experience, code quality, technical debt"
enable_discussions = false
+++

## The AI Diaries

> As soon as it works, no one calls it AI anymore - John McCarthy

So I tend to avoid using the term AI but it's sometimes unavoidable. Right now I am being forced to spend considerable time using coding tools. And sometimes I like it, sometimes I think it's a bore, and almost always it wastes some of my time. At a minimum it makes up for all the time it wastes but it always creates more noise than value. I have a lot of anecdotes working in this space so I will land them here, at the edge of obscurity.


## DevLog

<div class="devlog-entry">

## 28 01 2026

I gotta admit there is one thing about using AI coding tools that continues to be true no matter how much I try and constrain the model's failures I generally get similar results. If I don't know exactly what I want it to do and can provide a complex enough context the results will be that of an "Eager Intern" meaning I will get results that I didn't expect and when there were obvious places where the model should have stopped and asked questions it failed. I suspect that the model architecture was trained to focus more on task completion than task accuracy. I have a few times been able to get various agents to "give up" and tell me to try again. Of those Junie definitely does this and doesn't waste my time. Claude-Code though is too appeasing, it closes tasks without verification even when prompted to verify their work. Even with orchestration of multiple agents with fresh contexts, asking to build an app that isn't a todo-list will fail. This benefits the sale of coding tools, during evaluation it impresses with the ability to construct simple things but falls over when complex solutions are required. When I say complex I mean those that are generally novel or require doing interactions over APIs. It commonly produces boilerplate which I think is by design to influence the numbers for LoC for code generation stats. But insidiously, it is also there to obscure the solution it introduces.

A clear sign of AI code generation is bloat and intentional omissions. As of yet the only way I have found to avoid this omissions is to have the model show its work and put it in the clear view of me. So I can set it on a task and watch its completion, then ask it to review the goals and try again. This clearly sucks and I can introduce tools to guide it away from the problem but that's just a bad tool not something that is going to change the nature of my job. It is on the other hand an insult to my 20 year career and all the juniors that are unable to get a job because there is an assumption that if we just "trust me bro" enough it will work.

</div>

<div class="devlog-entry">

## 27 01 2026

> If you work for a company that laid off all your juniors in the past year, it is unbelievably poor taste to continue posting about the merits of AI and vibe coding on a platform where the majority of folks are currently looking for full-time work and do not want to be beaten to death with constant AI thinkpieces. Where did human-centered go in 2026? Because all I've seen so far from C-suite leaders and middle managers is forgetting how they got to where they are now. - Jen Udan - [REF](https://www.linkedin.com/posts/activity-7416144126259200000-3bt5?utm_source=share&utm_medium=member_desktop&rcm=ACoAAAIQ9iQBdQxO0rU7SDH3FYCQeNKWu3Zrg_A)

I have been thinking about it like this... consider some big enterprise makes this commitment, they have to get some financial approval for the act and may have committed to some outputs. Now let's say that AI is golf clubs and we just gave everyone a real nice set followed by, be good at golf by the end of the month. All this hype is just from people who own sporting goods stores. The latest debacle about cursor creating a browser without a human in the loop where it didn't compile and humans were in the loop still can land in the post truth world we live in. If my job was being told things are being accomplished and I get access to a todo-list that tells me my tasks are done it's gunna be real hard to not be attracted to such things.

I get to see the outputs of the C-Suite from time to time. The model tries to do the engineering work for me and guided by a visitor it often misses where the rules matter and where the rules can be bent.

![throughput-over-precision](../1_3iC6cilfUdvndZUVRELmBA.webp)

It's this ^ a very enticing concept. What of course is missed is I have to keep watching the bots work and stop them from looping. I guarantee it will get better but if the need for progress is all we care about maybe we should be thinking back to something simpler. People of Process, if we need to get things done we need to cut the red tape not unroll all the red tape into a ball and then wonder why we can't find anything.

</div>

<div class="devlog-entry">

## 20 01 2026

This one is more just the fun of working with other engineers and AI. While I will not post the code I was impacted by the size of the rebase it caused and the need for me to rewrite my feature. The code the model wrote only cared about things working. It built 200 line blocks of deeply nested conditional logic into existing functions adding catch clauses for exceptions that mean another service has failed and should not be caught. The telling part is when we reviewed the code with the developer he was unable to explain the why these things existed. It's a noob mistake but it's one that AI tends to promote. The endless "Trust Me Bro," and instead this wasted 6 hours of developer time and 3 days in a feature rewrite.

I know there is a mentality that encapsulation adds to cognitive overhead in humans but it exists because 5 levels of if statements is higher. But what happens when the same code was reviewed by the same model that produced it. The code seems to make sense and without the context of the architecture aka we just focused the changes on a single file we end up with some real new debt.

</div>
