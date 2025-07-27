from flask import Blueprint, render_template, current_app

from dashboard.db import get_db

bp = Blueprint("history", __name__)

@bp.route('/history', methods=('GET', 'POST'))
def history():
    db = get_db()
    history_data = db.execute( 'SELECT * FROM sensor_readings').fetchall()

    return render_template('history.html', history=history_data)

@bp.route('/history/upstairs', methods=('GET', 'POST'))
def upstairs():
    db = get_db()
    upstairs_topic = current_app.devices["top"].topic
    upstairs_data = db.execute ( 'SELECT * FROM sensor_readings where topic= ?',
                                (upstairs_topic,)).fetchall()

    return render_template('upstairs.html', history=upstairs_data)

@bp.route('/history/living_room', methods=('GET', 'POST'))
def living_room():
    db = get_db()
    living_room_topic = current_app.devices["middle"].topic
    living_room_data = db.execute ( 'SELECT * FROM sensor_readings where topic= ?',
                                (living_room_topic,)).fetchall()

    return render_template('living_room.html', history=living_room_data)

@bp.route('/history/laundry_room', methods=('GET', 'POST'))
def laundry_room():
    db = get_db()
    laundry_room_topic = current_app.devices["lower"].topic
    laundry_room_data = db.execute ( 'SELECT * FROM sensor_readings where topic= ?',
                                (laundry_room_topic,)).fetchall()

    return render_template('laundry_room.html', history=laundry_room_data)
