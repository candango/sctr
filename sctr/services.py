from cartola import security, sysexits
from firenado import service
import functools
import getpass
import sys


class UserService(service.FirenadoService):

    @property
    def app_component(self):
        return self.data_connected.get_app_component()

    def by_username(self, username):
        if "users" in self.app_component.conf:
            for user in self.app_component.conf['users']:
                if user['name'] == username:
                    return user
        return None

    def authenticate(self, username, password):
        user = self.by_username(username)
        if user is None:
            return False
        manager = security.Sha512KeyManager()
        return manager.validate(password, user['password'])

    def get_users(self):
        if "users" in self.app_component.conf:
            return self.app_component.conf['users']
        return None
