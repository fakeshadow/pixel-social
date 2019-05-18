import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:flutter_web/material.dart';
import 'package:pixel_flutter_web/blocs/ErrorBlocs.dart';
import 'package:pixel_flutter_web/components/NewTopicButton.dart';
import 'package:pixel_flutter_web/components/UserButton.dart';

class FloatingAppBar extends StatelessWidget {
  final String title;
  final Function onNewTopicButtonPressed;

  FloatingAppBar({this.title, this.onNewTopicButtonPressed});

  @override
  Widget build(BuildContext context) {
    return SliverAppBar(
      floating: true,
      snap: true,
      forceElevated: true,
      elevation: 5.0,
      title: Text(title),
      leading: IconButton(
        onPressed: () => BlocProvider.of<ErrorBloc>(context)
            .dispatch(GetSuccess(success: "You pressed something")),
        icon: Icon(Icons.apps),
      ),
      actions: <Widget>[
        NewTopicButton(onPressed: onNewTopicButtonPressed),
        UserButton()
      ],
    );
  }
}
