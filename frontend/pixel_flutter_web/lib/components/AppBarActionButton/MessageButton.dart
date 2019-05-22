import 'package:flutter_web/material.dart';

import 'package:flutter_bloc/flutter_bloc.dart';

import 'package:pixel_flutter_web/blocs/UserBlocs.dart';

import 'package:pixel_flutter_web/components/FloatingBarActionIcon.dart';

class MessageButton extends StatelessWidget {

  @override
  Widget build(BuildContext context) {
    return Hero(
        tag: 'checkMessage',
        child: Material(
            color: Colors.transparent,
            child: BlocBuilder(
              bloc: BlocProvider.of<UserBloc>(context),
              builder: (context, state) {
                if (state is UserLoaded) {
                  return FloatingBarActionIcon(
                      iconSize: 25,
                      icon: Icon(Icons.add_alert),
                      onPressed: (){});
                } else {
                  return Container();
                }
              },
            )));
  }
}