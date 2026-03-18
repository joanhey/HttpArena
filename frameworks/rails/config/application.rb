require 'rails'
require 'action_controller/railtie'

class BenchmarkApp < Rails::Application
  config.load_defaults 8.0
  config.eager_load = true
  config.api_only = true
  config.secret_key_base = 'benchmark-not-secret'
  config.hosts.clear
  config.consider_all_requests_local = false

  # Disable all middleware we don't need
  config.middleware.delete ActionDispatch::HostAuthorization
  config.middleware.delete ActionDispatch::Callbacks
  config.middleware.delete ActionDispatch::ActionableExceptions
  config.middleware.delete ActionDispatch::RemoteIp
  config.middleware.delete ActionDispatch::RequestId
  config.middleware.delete Rails::Rack::Logger
  config.middleware.delete ActionDispatch::ShowExceptions

  # Silence logging
  config.logger = Logger.new('/dev/null')
  config.log_level = :fatal
end
