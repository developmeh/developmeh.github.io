+++
title = "Decoupling Patterns in Ruby"
template = "page.html"
+++

## A Big Picture

Specifically, in my lifetime working with Ruby on Rails developers there has been a pattern I will describe as "Lazy Coupling". You will not find that pattern in the Gang of Four and if you google it don't get distracted by Loose Coupling.
I believe Sandi Metz covered this while describing SOLID in context of Dependency Injection with this [example](https://sandimetz.com/blog/2009/03/21/solid-design-principles#example4pain). While Sandi isn't intending to pick on the Rails world
I am not so kind. Its the world of clean code that Rails often violates. By its design is presents a house of "broken windows" to new developers and it takes considerable effort to break that dogma as they mature.

So lets talk about the big picture of decoupling and go a couple of steps beyond the Dependendency Injection and Dependency Inversion we might want to introduce to our applications.

I was once asked, "where would you put your business logic in a MVC application?"

As you can expect I sardonically responded. Not in the MVC application. That kind of impertanence requires some explanation so lets cool down for a moment and see if we can set some ground rules.

As Sandi explains DI isn't so scarry, infact none of SOLID is that scary as a set of principles they guide us away from code smells and towards code that is easier for humans to understand. We get close to that dream
of well abstracted, well tested components. So lets talk about a decoupled system architecture for building an API platform in Ruby on Rails.

The components of our system are not limited to the following and while some literally are represented within Rails they are also spiritual bondaries for logic. I like to consider the design from the same direction as execution,
allow us to assume we have a running Ruby on Rails application. It is a stack of middleware attached to Rack run by a threaded executor like Puma, standard.

Components:
- Controller
- Services
- Model (Data Access Layer)

Firstly, we have a __controller__ which has 1 job and only 1 job, to act as the programatic interface for the request and response. Think of it like a main function or an entrypoint for our script. Its context is,
parameters provided by the user-agent, context provided by the protocol and method, and intent provided by the route. The question you should be asking yourself here is;

> can I process the intent based on what has been provided?

Some things we must know here that represents our Buy-it-Now interface. The user we are acting upon and the product we are selling. At this stage of the request we are only interested if those values are present and how to respond if those values are malformed or omitted.
No other work should be done! The controller or entrypoint doesn't care if the user exists or if the product is in stock those concerns are for our service layer.

```ruby
class BuyItNowController < ApplicationController
  def initialize(
    idempotency_handler: IdempotencyResponseHandler.new,
    error_handler: BuyItNowErrorHandler.new,
    buy_it_now_service: BuyItNowService
  )
    @idempotency_handler = idempotency_handler
    @error_handler = error_handler
    super()
  end

  def buy_it_now
    buy_it_now_service.deliver(user_id: current_user, )

    # Update product logic here...
    render json: { message: 'Product updated successfully' }, status: :ok
  end

  def idempotency_params
    params.require(:idemptotency_id).permit(:idempotency_id)
  end

  def product_params
    params.require(:product).permit(:sku, :quantity)
  end
end
```

How did we get `current_user`
```ruby
# user_auth_middleware.rb
class UserAuthMiddleware
  def initialize(app)
    @app = app
  end

  def call(env)
    # valdiate session data or JWT here
    env["HTTP_X_PROCESSED_BY_MIDDLEWARE"] = "true"
    env
    @app.call(env)
  end
end

class UserAuthConstraint
  def matches?(request)
    request.headers["X-Processed-By-Middleware"] == "true"
  end
end


# routes.rb
constraints MiddlewareConstraint.new do
  # Routes that require the middleware processing
  resources :products
end
```

The request comes in and touches Puma, which creats a thread and populates the request context within Rack, at this point we have a Ruby hash with all the details from the network request. Much later we get to the controller
which represents the entry point of our API logic and because we have bespoke logic we need to provide a business case for its execution. This API in question is a Command that provides a "Buy-it-Now" feature for our dogfood ecommerce portal.
It will, assuming we are logged in, send a product using our default payment instrument to our default shipping location.

    HTTP REQUEST
    |
    PUMA
    |
    |--> Thread Spawned
    |    |
    |    |--> Convert HTTP Data to Ruby Hash
    |    |--> RACK
    |    |    |
    |    |    |--> Rack Middleware|    |    |--> Rails Middleware
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
