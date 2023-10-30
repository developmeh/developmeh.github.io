+++
title = "Decoupling Patterns in Ruby: Overview"
template = "page.html"
weight = 0
+++

## An Uncomplicated Picture

I was once asked, "Where would you put your business logic in an MVC application?"

As you can expect, I sardonically responded. Not in the MVC application. That impertinence requires some explanation, so let's cool down for a moment and see if we can set some ground rules.

As Sandi Metz kindly explains, DI isn't so scary; none of SOLID is, and as a set of principles, it guides us away from smells and towards code that is easier for humans to understand. We get closer to that dream of well-abstracted, isolation-tested components.

Specifically, in my lifetime working with Ruby on Rails developers, there has been a pattern I will describe as "Lazy Coupling." You will not find that pattern in the Gang of Four, and if you google it, don't get distracted by Loose Coupling. I believe Sandi Metz covered this while describing SOLID in the context of Dependency Injection with this [example](https://sandimetz.com/blog/2009/03/21/solid-design-principles#example4pain). While Sandi isn't intending to pick on the Rails world, I am not so kind. It's the world of clean code that Rails often violates. By its design, it presents a house of "broken windows" to new developers, and it takes considerable effort to break that dogma as they mature.

__Lazy Coupling__ is when we directly assign a constant to the return value of an instance method of another class. It looks a little something like this:
<table style="width:100%">
<thead>
<tr>
<th style="width:50%">
Without DI
</th>
<th>
With DI
</th>
</tr>
</thead>
<tbody>
<tr>
<td>

```ruby
class LazyCouple
  def perform
    data = data_source(1)
    data.name
  end

private
  def data_source(id)
    TheDataSource.find(id)
  end
end
```
</td>
<td>

```ruby
class LooseCouple
  def initialize(data_source = TheDataSource)
    @data_source = data_source
  end

  def perform
    data = @data_source.find(1)
    data.name
  end
end
```
</td>
</tr>
<tr>
<td>

```ruby
RSpec.describe LazyCouple do
  describe '#perform' do
    let(:data_source_instance) do
      double('DataSourceInstance', name: 'Some Name')
    end

    before do
      allow(TheDataSource).to receive(:find)
        .with(1).and_return(data_source_instance)
    end

    subject { described_class.new.perform }

    it 'fetches data and returns the name' do
      expect(subject).to eq('Some Name')
    end
  end
end
```  
</td>
<td>

```ruby
RSpec.describe LooseCouple do
  describe '#perform' do
    let(:data_source_instance) do
	double('DataSourceInstance', name: 'Some Name')
    end
    let(:mock_data_source) do
	double('MockDataSource', find: data_source_instance)
    end
    
    subject { described_class.new(mock_data_source).perform }

    it 'fetches data and returns the name' do
      expect(subject).to eq('Some Name')
    end
  end
end
```
</td>
</tr>
</tbody>
</table>

These two look nearly identical, but can you spot the big difference? It's in the specs, which often are our best mirror on implementation. Our Spec doesn't need to reference the constant for TheDataSource; it instead provides its own mock, and that mock is a double.

I hear you saying, "Big deal!"

It is a BIG DEAL!

These little changes add up. The DI test is a little easier to read as it references more of its own constants. It is also completely isolated from the system. If you need to refactor this code between gems, this test could be transported along, and we can guarantee that our coverage and quality don't degrade.

Without going too far out on a limb, we have provided a space for this class to be Open/Closed; we can extend its behavior without modifying the class. Say we decided to change the data source. We may want to continue to use this behavior, but we have been developing a new data source that ActiveRecord does not support, like a network call. This class can stay the same, and our Spec will still validate this behavior in isolation. It also provided documentation of our protocol with the dependency.

It's important to recall that Ruby communicates over __Protocols__ not __Contracts__ by default. Each implementation of a new data source is backed by its integration test while the core functionality continues its life Closed.

Listen, I can hear the chant in the background slowly growing... yagni... Yagni... YAGni... YAGNI. The truth is I agree with you, and it's a balancing act. I have always said that our job as engineers is to manage the change in our systems and not necessarily write code.
Consider this quote from Robert Nystrom.

> There's no easy answer here. Making your program more flexible so you can prototype faster will have some performance cost. Likewise, optimizing your code will make it less flexible.

Interestingly, Robert identifies that a more rigid software architecture free from additional abstractions is the pre-optimization. We trade off current performance for flexibility. Of course, the point concerns systems performance, but it also applies to delivery performance. It takes a little longer to design and implement a flexible architecture, and we get that payback in agility when changing the system later.

To explore these trade-offs, let's discuss a decoupled system architecture for building an API platform in Ruby on Rails.

The components of our system are not limited to the following, and while some literally are represented within Rails, they are also spiritual boundaries for logic. I prefer to consider software architecture design from the same direction as execution, allowing us to assume we have a running Ruby on Rails application. It is a stack of middleware attached to Rack run by a threaded executor like Puma, standard.

Components:
- Controller (Protocol Handler)
- Services (Business Logic)
- Model (Data Access Layer)

The request comes in and touches Puma, which creates a thread and populates the request context within Rack; at this point, we have a Ruby hash with all the details from the network request. Much later, we get to the controller, which represents the entry point of our API logic, and because we have bespoke logic, we need to provide a business case for its execution. This API in question is a Command that offers a "Buy-it-Now" feature for our dog food e-commerce portal. It will, assuming the actor is logged in, send a product using our default payment instrument to our default shipping location.

    HTTP REQUEST
    |
    PUMA
    |
    |--> Thread Spawned
    |    |
    |    |--> Convert HTTP Data to Ruby Hash
    |    |--> RACK
    |    |    |
    |    |    |--> Rack Middleware
    |    |    |--> Rails Middleware
    |    |    |    |
    |    |    |    |--> Routing Middleware -> Create Controller Instance
    |    |    |    |    |
    |    |    |    |    |--> Controller Action
    |    |    |    |    |--> __YOUR CODE__
    |    |    |    |<-- |<-- Response
    |    |    |    |--> Other Middlewares (if any)
    |<-- |<-- |<-- Return Hash to PUMA
    |
    HTTP RESPONSE

As you can imagine, there are many opportunities for decoupling that Rails appears to want to fight us on but, in fact, happily supports. Our first stop will be the Protocol Handler or, more commonly, the controller. In our next part, we will describe the SOLID implication of the controller and design a pattern for building the entry point for any request, be that consumed by:
- API (Application Programming Interface)
- LPC (Local Procedure Call)
- deferred worker
- ESB (Enterprise Service Bus)
- RPC (Remote Procedure Call)

From there, we will move on to the design of our Service Layer; this will include observability concerns as well as auditing and Actors. We will, of course, always keep this in the context of enterprise production systems; we will enforce security and implement RBAC (Role Based Access Control). Our practice will also include a deeper dive into relevant programming patterns outside those introduced by SOLID.
