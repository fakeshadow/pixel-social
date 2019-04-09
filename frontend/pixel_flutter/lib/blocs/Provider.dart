import 'package:flutter/material.dart';
import 'package:pixel_flutter/blocs/AuthBloc/AuthBloc.dart';

class Provider extends InheritedWidget {
  final bloc = AuthBloc();

  Provider({Key key, Widget child}) : super(key: key, child: child);

  bool updateShouldNotify(_) => true;

  static AuthBloc of(BuildContext context) {
    return (context.inheritFromWidgetOfExactType(Provider) as Provider).bloc;
  }
}