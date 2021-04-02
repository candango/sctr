from firenado import service


class UserService(service.FirenadoService):

    @property
    def app_component(self):
        return self.data_connected.get_app_component()

    def by_username(self, username):
        conf = self.component.conf
        print(conf)

    def get_users(self):
        if "users" in self.app_component.conf:
            return self.app_component.conf['users']
        return None
