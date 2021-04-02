from . import handlers
from firenado import tornadoweb


class SctrComponent(tornadoweb.TornadoComponent):

    def get_handlers(self):
        return [
            (r'/', handlers.IndexHandler),
        ]

    def get_config_filename(self):
        return "sctr"
