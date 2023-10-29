+++
title = "Decoupling Patterns in Ruby: Part 1 The Protocol Handler"
template = "page.html"
+++

## The Protocol Hander (The Controller)

Firstly, we have a __controller__, which has one job and only one job (ADD NOTE ABOUT SRP), to act as the programmatic interface for the request and response. Think of it as a primary function or an entry point for our script. Its context is
parameters provided by the user-agent, context provided by the protocol and method, and intent provided by the route. The question you should be asking yourself here is;

> Can I process the intent based on what has been provided by the request?

There are some things we must know here that represent our Buy-it-Now interface. The user we are acting upon and the product we are selling. At this stage of the request, we are only interested if those values are present and how to respond if those values are malformed or omitted.
The controller should not do any other work! The controller or entry point doesn't care if the user exists or the product is in stock; those concerns are for our service layer.

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
