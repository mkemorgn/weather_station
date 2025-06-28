import os

from flask import Flask


def create_app(test_config=None):
    # create and configure the app
    app = Flask(__name__, instance_relative_config=True)

    from . import dashboard, db, history
    db.init_app(app)
    app.register_blueprint(dashboard.bp)
    app.register_blueprint(history.bp)


    from .mqtt import MQTTDevice
    app.devices = {
        "top": MQTTDevice("upstairs", topic="D8132A722354/sensor_data", app=app),
        "middle": MQTTDevice("middle", topic="C4DEE25BA558/sensor_data", app=app),
        "lower": MQTTDevice("lower", topic="C82E1826BC40/sensor_data", app=app),
    }

    app.config.from_mapping(
        SECRET_KEY='dev',
        DATABASE=os.path.join(app.instance_path, 'dashboard.sqlite'),
    )

    if test_config is None:
        # load the instance config, if it exists, when not testing
        app.config.from_pyfile('config.py', silent=True)
    else:
        # load the test config if passed in
        app.config.from_mapping(test_config)

    # ensure the instance folder exists
    try:
        os.makedirs(app.instance_path)
    except OSError:
        pass

    return app
