# dashboard.py

from flask import Blueprint, jsonify, render_template, Response, stream_with_context, current_app
import time, json

bp = Blueprint("dashboard", __name__)

# Devices will be created and attached to app in create_app()

@bp.route("/")
def index():
    return render_template("index.html")

@bp.route("/data")
def get_data():
    return jsonify({
        "top": current_app.devices["top"].get_data(),
        "middle": current_app.devices["middle"].get_data(),
        "lower": current_app.devices["lower"].get_data()
    })

@bp.route("/stream")
def stream():
    def event_stream():
        while True:
            data = {
                "top": current_app.devices["top"].get_data(),
                "middle": current_app.devices["middle"].get_data(),
                "lower": current_app.devices["lower"].get_data(),
            }
            yield f"data: {json.dumps(data)}\n\n"
            time.sleep(1)

    return Response(stream_with_context(event_stream()), mimetype="text/event-stream")

