import 'dart:io';

import 'package:flutter/foundation.dart';
import 'package:web_socket_channel/io.dart';

import '../env.dart';

TalkAPI sockets = TalkAPI();

class TalkAPI {
  static final TalkAPI _sockets = TalkAPI._internal();

  factory TalkAPI() {
    return _sockets;
  }

  bool isManualClosed = false;

  TalkAPI._internal();

  IOWebSocketChannel _channel;

  ObserverList<Function> _listeners = ObserverList<Function>();

  connect() {
    _channel = IOWebSocketChannel.connect(env.WS_URL);
    isManualClosed = false;
  }

  close() {
    if (_channel != null) {
      if (_channel.sink != null) {
        _channel.sink.close();
        isManualClosed = true;
      }
    }
  }

  send(String message) async {
    if (_channel != null) {
      if (_channel.sink != null) {
        _channel.sink.add(message);
      }
    }
  }

  addListener(Function callback) {
    _listeners.add(callback);
  }

  removeListener(Function callback) {
    _listeners.remove(callback);
  }

  // retry connection every 3 seconds after disconnection
  handleConn(Function callback) {
    connect();
    _channel.stream.listen(_onMsg, onError: _onErr, onDone: () async {
      if (isManualClosed == false) {
        await Future.delayed(Duration(seconds: 3));
        handleConn(callback);
      }
    });
    callback();
  }

  _onMsg(msg) {
    _listeners.forEach((Function callback) {
      callback(msg);
    });
  }

  _onErr(msg) {
    _listeners.forEach((Function callback) {
      callback(msg);
    });
  }
}
