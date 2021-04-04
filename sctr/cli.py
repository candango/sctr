from . import services
from cartola import sysexits
from firenado import service
import functools
from getpass import getpass, getuser
import sys


def authenticated(method):
    """ Decorates a method to continue only if the user is authenticated
    """

    @service.served_by(services.UserService)
    @functools.wraps(method)
    def wrapper(self, namespace):
        username = namespace.user
        password = namespace.password
        print(namespace)
        if username is None:
            username = input("Username:")
        # if s
        if password is None:
            password = getpass("Password:")
        if not self.user_service.authenticate(username, password):
            print("ERROR: Invalid user. Please provide valid credentials.")
            print(namespace.help)
            sys.exit(sysexits.EX_NOUSER)
        return method(self, namespace)
    return wrapper
