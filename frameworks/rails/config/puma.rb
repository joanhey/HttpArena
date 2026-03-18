require 'etc'

cores = Etc.nprocessors
workers cores
threads 4, 4

bind 'tcp://0.0.0.0:8080'

preload_app!

before_fork do
  # Close any inherited DB connections
end
