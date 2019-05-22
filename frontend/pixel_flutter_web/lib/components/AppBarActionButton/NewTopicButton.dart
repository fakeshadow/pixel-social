import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:flutter_web/material.dart';

import 'package:pixel_flutter_web/env.dart';

import 'package:pixel_flutter_web/blocs/UserBlocs.dart';

import 'package:pixel_flutter_web/components/FloatingBarActionIcon.dart';

class NewTopicButton extends StatelessWidget with env {
  final Function onPressed;

  NewTopicButton({@required this.onPressed});

  @override
  Widget build(BuildContext context) {
    return Hero(
        tag: 'newTopic',
        child: Material(
            color: Colors.transparent,
            child: BlocBuilder(
              bloc: BlocProvider.of<UserBloc>(context),
              builder: (context, state) {
                if (state is UserLoaded) {
                  return FloatingBarActionIcon(
                      iconSize: 25,
                      icon: Icon(Icons.border_color),
                      onPressed: onPressed);
                } else {
                  return Container();
                }
              },
            )));
  }
}
