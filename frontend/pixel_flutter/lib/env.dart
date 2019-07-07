class env {

  /// main service ip
  final String url = 'http://192.168.1.197:3200/';

  static const String WS_URL = 'ws://192.168.1.197:3200/talk';

  /// how often user can fetch categories data
  final int TIME_GATE = 360000;

  /// Max length of topic title
  final int MAX_TITLE_LENGTH = 256;

  /// Max length of topic body of post content
  final int MAX_TEXT_LENGTH = 9999;

  /// breakpoint between lg and md size
  final double BREAK_POINT_WIDTH = 1000.0;

  /// breakpoint between web and mobile
  final double BREAK_POINT_WIDTH_SM = 600.0;

  /// Snackbar message for success input
  final String GOT_TOPIC = 'Modified topic success';
}