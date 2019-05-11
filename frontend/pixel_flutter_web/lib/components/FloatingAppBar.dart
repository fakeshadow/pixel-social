import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:flutter_web/material.dart';
import 'package:pixel_flutter_web/blocs/ErrorBlocs.dart';
import 'package:pixel_flutter_web/components/UserButton.dart';

class FloatingAppBar extends StatelessWidget {
  @override
  Widget build(BuildContext context) {
    return SliverAppBar(
      floating: true,
      snap: true,
      forceElevated: true,
      elevation: 5.0,
      title: Text("Pixel fultter web test"),
      leading: IconButton(
        onPressed: () => BlocProvider.of<ErrorBloc>(context)
            .dispatch(GetSuccess(success: "You pressed something")),
        icon: Icon(Icons.apps),
      ),
      actions: <Widget>[
        UserButton()
      ],
    );
  }
}
