import 'package:flutter/material.dart';

class HistoryLimit extends NavigatorObserver {
  final int limit;
  final history = <Route>[];

  HistoryLimit([this.limit]);

  @override
  void didReplace({Route<dynamic> newRoute, Route<dynamic> oldRoute}) {
    final index = history.indexOf(oldRoute);
    history[index] = newRoute;
  }

  @override
  void didPush(Route route, Route previousRoute) {
    history.add(route);
    if (history.length >= limit) {
      print(history);
      this.navigator.removeRoute(history.first);
    }
  }

  @override
  void didPop(Route route, Route previousRoute) {
    history.remove(route);
  }

  @override
  void didRemove(Route route, Route previousRoute) {
    history.remove(route);
  }
}
