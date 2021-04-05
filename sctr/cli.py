from . import services
from cartola import sysexits
from firenado import service
from firenado.util.argparse_util import FirenadoArgumentError
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
        if username:
            namespace.system=False
        if namespace.system:
            username = getuser()
        if username is None:
            username = input("Username:")
        if password is None:
            password = getpass("Password:")
        if not self.user_service.authenticate(username, password):
            print("ERROR: Invalid user. Please provide valid credentials.")
            sys.exit(sysexits.EX_NOUSER)
        self._user = self.user_service.by_username(username)
        return method(self, namespace)
    return wrapper
