from testapp import handlers
from firenado import tornadoweb


class TestappComponent(tornadoweb.TornadoComponent):

    def get_handlers(self):
        return [
            (r'/', handlers.IndexHandler),
        ]
